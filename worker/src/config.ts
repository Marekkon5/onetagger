export interface WorkerConfig {
  apiBase: string;
  secret: string;
  workerId: string;
  pollIntervalMs: number;
  heartbeatMs: number;
  storage: {
    driver: 'local' | 's3';
    localRoot: string;
    s3Endpoint?: string;
    s3Bucket?: string;
    s3AccessKey?: string;
    s3SecretKey?: string;
  };
}

function env(name: string, fallback = ''): string {
  return process.env[name] ?? fallback;
}

export function loadConfig(): WorkerConfig {
  const secret = env('IMPORT_WORKER_SECRET');
  if (!secret) throw new Error('IMPORT_WORKER_SECRET is required');
  const driver = env('IMPORT_STORAGE_DRIVER', 'local');
  if (driver !== 'local' && driver !== 's3') throw new Error('IMPORT_STORAGE_DRIVER must be local or s3');
  if (driver === 's3' && !(env('IMPORT_S3_ENDPOINT') && env('IMPORT_S3_BUCKET') && env('IMPORT_S3_ACCESS_KEY') && env('IMPORT_S3_SECRET_KEY'))) {
    throw new Error('IMPORT_S3_* configuration is required for the s3 storage driver');
  }
  return {
    apiBase: env('IMPORT_API_BASE', 'http://127.0.0.1:8787').replace(/\/$/, ''),
    secret,
    workerId: env('IMPORT_WORKER_ID', `worker-${process.pid}`),
    pollIntervalMs: Number(env('IMPORT_POLL_INTERVAL_MS', '2000')),
    heartbeatMs: Number(env('IMPORT_HEARTBEAT_MS', '20000')),
    storage: {
      driver,
      localRoot: env('IMPORT_STORAGE_LOCAL_ROOT', './data/imports'),
      s3Endpoint: env('IMPORT_S3_ENDPOINT') || undefined,
      s3Bucket: env('IMPORT_S3_BUCKET') || undefined,
      s3AccessKey: env('IMPORT_S3_ACCESS_KEY') || undefined,
      s3SecretKey: env('IMPORT_S3_SECRET_KEY') || undefined,
    },
  };
}
