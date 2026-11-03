import { createHash, randomUUID } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync, lstatSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { catalogMediaDirectory } from './media.js';
import type { CatalogRepository, StoredMediaAsset } from './repository.js';

interface Manifest {version:1;files:Record<string,string>}
const hash=(path:string)=>createHash('sha256').update(readFileSync(path)).digest('hex');
function verifyDatabase(path:string):StoredMediaAsset[] {
  const db=new DatabaseSync(path,{readOnly:true});
  try {
    const integrity=db.prepare('PRAGMA integrity_check').get() as {integrity_check:string};
    if(integrity.integrity_check!=='ok'||db.prepare('PRAGMA foreign_key_check').get())throw new Error('Catalog backup database integrity check failed');
    return (db.prepare('SELECT media_id,record_id,relative_path,byte_size FROM media_assets').all() as Array<{media_id:string;record_id:string;relative_path:string;byte_size:number}>).map(row=>({mediaId:row.media_id,recordId:row.record_id,relativePath:row.relative_path,byteSize:row.byte_size,role:'waveform',mimeType:'image/png',sourceUrl:''}));
  }finally{db.close();}
}
function safeMediaPath(asset:StoredMediaAsset):string {
  if(!/^[A-Za-z0-9_-]{8,128}\.png$/.test(asset.relativePath)||asset.relativePath!==`${asset.mediaId}.png`)throw new Error('Catalog backup media path is invalid');
  return `media/${asset.relativePath}`;
}
function verifyBundle(source:string):Manifest {
  const manifest=JSON.parse(readFileSync(join(source,'manifest.json'),'utf8')) as Manifest;
  if(manifest.version!==1||!manifest.files||typeof manifest.files!=='object'||!manifest.files['catalog.sqlite'])throw new Error('Catalog backup manifest is invalid');
  for(const [name,expected] of Object.entries(manifest.files)) {
    if(name!=='catalog.sqlite'&&!/^media\/[A-Za-z0-9_-]{8,128}\.png$/.test(name))throw new Error('Catalog backup manifest path is invalid');
    const path=join(source,name);if(!lstatSync(path).isFile()||hash(path)!==expected)throw new Error('Catalog backup file checksum does not match');
  }
  const assets=verifyDatabase(join(source,'catalog.sqlite'));
  for(const asset of assets){const name=safeMediaPath(asset);if(!manifest.files[name]||lstatSync(join(source,name)).size!==asset.byteSize)throw new Error('Catalog backup media integrity check failed');}
  return manifest;
}

/** Writes a verified directory bundle containing a consistent SQLite snapshot and its immutable media. */
export async function backupCatalog(repo:CatalogRepository,destination:string):Promise<void> {
  const target=resolve(destination);if(existsSync(target))throw new Error('Backup destination must be a new directory');
  mkdirSync(dirname(target),{recursive:true,mode:0o700});const temporary=`${target}.${randomUUID()}.tmp`;mkdirSync(temporary,{mode:0o700});
  try {
    await repo.backupTo(join(temporary,'catalog.sqlite'));
    const assets=verifyDatabase(join(temporary,'catalog.sqlite')),manifest:Manifest={version:1,files:{'catalog.sqlite':hash(join(temporary,'catalog.sqlite'))}};
    mkdirSync(join(temporary,'media'),{mode:0o700});
    for(const asset of assets){const name=safeMediaPath(asset),source=join(catalogMediaDirectory(),asset.relativePath);if(!lstatSync(source).isFile()||lstatSync(source).size!==asset.byteSize)throw new Error('Catalog backup source media integrity check failed');copyFileSync(source,join(temporary,name));manifest.files[name]=hash(join(temporary,name));}
    writeFileSync(join(temporary,'manifest.json'),JSON.stringify(manifest,null,2),{mode:0o600,flag:'wx'});verifyBundle(temporary);renameSync(temporary,target);
  }finally{rmSync(temporary,{recursive:true,force:true});}
}
/** Restores to a new directory; the caller starts a worker using its catalog.sqlite and media paths. */
export async function restoreCatalog(source:string,destination:string):Promise<void> {
  const backup=resolve(source),target=resolve(destination);if(existsSync(target)||target===backup)throw new Error('Restore destination must be a new directory; existing data cannot be overwritten');
  const manifest=verifyBundle(backup);mkdirSync(dirname(target),{recursive:true,mode:0o700});const temporary=`${target}.${randomUUID()}.tmp`;mkdirSync(join(temporary,'media'),{recursive:true,mode:0o700});
  try {for(const name of Object.keys(manifest.files))copyFileSync(join(backup,name),join(temporary,name));writeFileSync(join(temporary,'manifest.json'),JSON.stringify(manifest),{mode:0o600});verifyBundle(temporary);renameSync(temporary,target);}
  finally{rmSync(temporary,{recursive:true,force:true});}
}
