import { createHash, randomUUID } from 'node:crypto';
import { mkdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { inflateSync } from 'node:zlib';
import type { CatalogRecord, MixRecord, RecordId } from '@syco23/catalog-domain';
import { CatalogRecordSchema } from '@syco23/catalog-domain';
import type { CatalogRepository, StoredMediaAsset } from './repository.js';

const maxPngBytes = 8 * 1024 * 1024;

export function catalogMediaDirectory(): string {
  return process.env.CATALOG_MEDIA_PATH
    || join(dirname(process.env.CATALOG_DB_PATH || './data/catalog.sqlite'), 'media');
}

function validatePng(png: Uint8Array): { width: number; height: number } {
  if (png.byteLength > maxPngBytes) throw new Error('Waveform PNG exceeds the 8 MiB size limit');
  const bytes = Buffer.from(png);
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  if (bytes.length < 24 || !bytes.subarray(0, 8).equals(signature) || bytes.toString('ascii', 12, 16) !== 'IHDR') {
    throw new Error('Waveform image must be a valid PNG with an IHDR header');
  }
  const width = bytes.readUInt32BE(16);
  const height = bytes.readUInt32BE(20);
  if (!width || !height || width > 8192 || height > 8192 || width * height > 16_000_000) {
    throw new Error('Waveform PNG dimensions exceed the supported bounds');
  }
  let offset=8,ended=false,seenHeader=false,seenData=false,palette=false;
  let bitDepth=0,colorType=0;
  const chunks:Buffer[]=[];
  const crc32=(data:Buffer)=>{let crc=0xffffffff;for(const byte of data){crc^=byte;for(let bit=0;bit<8;bit++)crc=(crc>>>1)^((crc&1)?0xedb88320:0);}return (crc^0xffffffff)>>>0;};
  while(offset<bytes.length) {
    if(offset+12>bytes.length)throw new Error('Waveform PNG chunk is truncated');
    const size=bytes.readUInt32BE(offset),type=bytes.toString('ascii',offset+4,offset+8),end=offset+12+size;
    if(end>bytes.length||crc32(bytes.subarray(offset+4,offset+8+size))!==bytes.readUInt32BE(offset+8+size))throw new Error('Waveform PNG chunk checksum is invalid');
    if(!seenHeader&&type!=='IHDR')throw new Error('Waveform PNG must start with IHDR');
    if(type==='IHDR') {if(seenHeader||size!==13)throw new Error('Waveform PNG IHDR is invalid');seenHeader=true;bitDepth=bytes[offset+16];colorType=bytes[offset+17];if(bytes[offset+18]||bytes[offset+19]||bytes[offset+20])throw new Error('Waveform PNG encoding is unsupported');const allowed:Record<number,number[]>={0:[1,2,4,8,16],2:[8,16],3:[1,2,4,8],4:[8,16],6:[8,16]};if(!allowed[colorType]?.includes(bitDepth))throw new Error('Waveform PNG color format is invalid');}
    else if(type==='IDAT') {seenData=true;chunks.push(bytes.subarray(offset+8,offset+8+size));}
    else if(type==='PLTE') {if(!size||size%3||size>768)throw new Error('Waveform PNG palette is invalid');palette=true;}
    else if(type==='IEND') {if(size||!seenData||end!==bytes.length)throw new Error('Waveform PNG end chunk is invalid');ended=true;}
    else if(type[0]===type[0].toUpperCase())throw new Error('Waveform PNG critical chunk is unsupported');
    offset=end;
  }
  if(!ended||(colorType===3&&!palette))throw new Error('Waveform PNG is incomplete');
  const channels=({0:1,2:3,3:1,4:2,6:4} as Record<number,number>)[colorType];
  const rowBytes=Math.ceil(width*channels*bitDepth/8),expected=(rowBytes+1)*height;
  if(expected>64*1024*1024)throw new Error('Waveform PNG decoded size exceeds supported bounds');
  try {const decoded=inflateSync(Buffer.concat(chunks),{maxOutputLength:expected});if(decoded.length!==expected)throw new Error();for(let row=0;row<height;row++)if(decoded[row*(rowBytes+1)]>4)throw new Error();}
  catch {throw new Error('Waveform PNG pixel data is invalid');}
  return { width, height };
}

function validSourceUrl(sourceUrl: string): string {
  const url = new URL(sourceUrl);
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) throw new Error('Waveform source URL is invalid');
  return url.toString();
}

function mediaIdFor(recordId: RecordId): RecordId {
  return `media_${createHash('sha256').update(recordId).digest('hex').slice(0, 32)}`;
}

export function persistWaveform(repository: CatalogRepository, mixId: RecordId, png: Uint8Array, sourceUrl: string): CatalogRecord {
  const dimensions = validatePng(png);
  const canonicalSourceUrl = validSourceUrl(sourceUrl);
  const current = repository.getRecord(mixId);
  if (!current || current.kind !== 'mix') throw new Error('Waveform target must be an existing mix');
  const existing = current.assets.find((asset) => asset.role === 'waveform');
  if (existing) return current;

  const mediaId = mediaIdFor(mixId);
  const relativePath = `${mediaId}.png`;
  const mediaDirectory = catalogMediaDirectory();
  mkdirSync(mediaDirectory, { recursive: true, mode: 0o700 });
  const destination = join(mediaDirectory, relativePath);
  const temporary = join(mediaDirectory, `.${mediaId}.${randomUUID()}.tmp`);
  writeFileSync(temporary, png, { flag: 'wx', mode: 0o600 });

  try {
    renameSync(temporary, destination);
    const asset = {
      role: 'waveform' as const,
      url: `/api/catalog/media/${mediaId}`,
      source: 'local' as const,
      width: dimensions.width,
      height: dimensions.height,
      mediaId
    };
    const updated: MixRecord = {
      ...current,
      assets: [...current.assets, asset],
      revision: current.revision + 1,
      updatedAt: new Date().toISOString()
    };
    repository.transaction((tx) => {
      tx.saveRecord(updated, current.revision);
      tx.addMediaAsset({ mediaId, recordId: mixId, role: 'waveform', relativePath, mimeType: 'image/png', byteSize: png.byteLength, sourceUrl: canonicalSourceUrl });
    });
    return CatalogRecordSchema.parse(updated);
  } catch (error) {
    rmSync(destination, { force: true });
    throw error;
  } finally {
    rmSync(temporary, { force: true });
  }
}

export function readWaveformMedia(repository: CatalogRepository, mediaId: RecordId): Buffer | undefined {
  const asset: StoredMediaAsset | undefined = repository.getMediaAsset(mediaId);
  if (!asset || asset.role !== 'waveform' || basename(asset.relativePath) !== asset.relativePath) return undefined;
  const record = repository.getRecord(asset.recordId);
  if (!record || record.kind !== 'mix' || !record.assets.some((item) => item.mediaId === mediaId && item.role === 'waveform')) return undefined;
  const filePath = join(catalogMediaDirectory(), asset.relativePath);
  try {
    const stats = statSync(filePath);
    if (!stats.isFile() || stats.size !== asset.byteSize || stats.size > maxPngBytes) return undefined;
    const bytes = readFileSync(filePath);
    validatePng(bytes);
    return bytes;
  } catch {
    return undefined;
  }
}