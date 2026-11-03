import type { ProviderId } from '../domain/types';

/**
 * Presentation-only vocabulary for the archive UI. Machine identifiers and enums stay
 * in the data layer; these maps translate them into wording a non-technical reader can
 * trust. Every label lives here so a state or field can never be shown as a raw key or
 * as bare JSON anywhere in the client.
 */

export const PROVIDER_LABELS: Record<ProviderId, string> = {
  freeteknomusic: 'Freeteknomusic',
  soundcloud: 'SoundCloud',
  archiveorg: 'Archive.org',
  discogs: 'Discogs',
  youtube: 'YouTube',
  hearthis: 'hearthis.at',
  mixcloud: 'Mixcloud'
};

export const providerLabel = (id: string): string => PROVIDER_LABELS[id as ProviderId] ?? id;

export const KIND_LABELS: Record<string, string> = {
  mix: 'Recording',
  entity: 'Artist · crew · label',
  event: 'Event'
};

export const VERIFICATION_LABELS: Record<string, string> = {
  proposed: 'Proposed',
  'source-confirmed': 'Source confirmed',
  'curator-confirmed': 'Curator confirmed'
};

export const REVIEW_STATE_LABELS: Record<string, string> = {
  ready: 'Ready',
  review: 'Needs review'
};

export const JOB_STATE_LABELS: Record<string, string> = {
  queued: 'Queued',
  running: 'Searching…',
  review: 'Awaiting your review',
  done: 'Finished',
  error: 'Failed',
  cancelled: 'Stopped'
};

export const RUN_STATE_LABELS: Record<string, string> = {
  running: 'In progress',
  completed: 'Completed',
  interrupted: 'Interrupted',
  failed: 'Failed'
};

export const ANALYSIS_STATE_LABELS: Record<string, string> = {
  queued: 'Queued',
  running: 'Analyzing…',
  done: 'Done',
  error: 'Failed',
  interrupted: 'Interrupted'
};

export const HEALTH_STATE_LABELS: Record<string, string> = {
  ready: 'Ready',
  limited: 'Limited',
  offline: 'Unavailable'
};

export const PROVIDER_MODE_LABELS: Record<string, string> = {
  discover: 'Finds new mixes',
  enrich: 'Fills missing fields',
  both: 'Finds & fills'
};

export const PROVIDER_AUTH_LABELS: Record<string, string> = {
  none: 'No login needed',
  required: 'Needs API keys',
  recommended: 'Optional API key',
  optional: 'Optional'
};

export const EVIDENCE_LABELS: Record<string, string> = {
  direct: 'Direct from source',
  parsed: 'Parsed from source',
  curated: 'Curator decision',
  analysis: 'Audio analysis'
};

export const DISPOSITION_LABELS: Record<string, string> = {
  selected: 'Selected',
  corroborated: 'Confirmed by sources',
  pending: 'Awaiting decision',
  rejected: 'Rejected'
};

export const LOCAL_TRACK_STATE_LABELS: Record<string, string> = {
  scanned: 'Scanned',
  enriched: 'Enriched',
  skipped: 'Needs a curator',
  error: 'Failed',
  written: 'Copy saved'
};

export const MIGRATION_STATUS_LABELS: Record<string, string> = {
  imported: 'Imported',
  existing: 'Already present',
  rejected: 'Excluded',
  partial: 'Partially imported'
};

/**
 * Field keys are internal identifiers; render them as readable labels. Covers the keys
 * produced by `missingFields()`, the field definitions and every claim/evidence field.
 */
export const FIELD_LABELS: Record<string, string> = {
  title: 'Title',
  description: 'Description',
  durationMs: 'Duration',
  recordingDate: 'Recording date',
  genres: 'Genres',
  styles: 'Styles',
  cover: 'Cover art',
  people: 'Linked people',
  eventIds: 'Events',
  playbackUrls: 'Playback links',
  sources: 'Sources',
  displayName: 'Display name',
  roles: 'Roles',
  aliases: 'Aliases',
  profile: 'Profile',
  country: 'Country',
  realName: 'Real name',
  groupIds: 'Groups',
  memberIds: 'Members',
  parentId: 'Parent label',
  subLabelIds: 'Sub-labels',
  websiteUrls: 'Websites',
  providerRefs: 'Provider identities',
  artistPortrait: 'Artist portrait',
  crewLogo: 'Crew logo',
  labelLogo: 'Label logo',
  name: 'Name',
  eventDate: 'Event date',
  startDate: 'Start date',
  endDate: 'End date',
  venue: 'Venue',
  locality: 'Locality',
  sourceUrls: 'Source links',
  mixIds: 'Mixes',
  flyer: 'Flyer',
  possibleDuplicate: 'Possible duplicate',
  assets: 'Images and audio',
  artists: 'Performers',
  crews: 'Crews',
  labels: 'Labels',
  events: 'Event',
  artist: 'Artist details',
  crew: 'Crew details',
  label: 'Label details'
};

export function fieldLabel(key: string): string {
  if (FIELD_LABELS[key]) return FIELD_LABELS[key];
  return key
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/[._]+/g, ' ')
    .replace(/\b\w/g, (character) => character.toUpperCase())
    .trim() || key;
}

function shortUrl(value: string): string {
  try {
    const parsed = new URL(value);
    const host = parsed.hostname.replace(/^www\./, '');
    const tail = parsed.pathname.split('/').filter(Boolean).slice(-1)[0] || '';
    return tail ? `${host}/…/${tail}` : host;
  } catch {
    return value;
  }
}

/**
 * Human-facing value rendering. Never falls back to JSON: objects become short readable
 * summaries, dates keep their precision, and long URLs are shortened to their origin.
 */
export function humanValue(value: unknown): string {
  if (value === undefined || value === null || value === '' || (Array.isArray(value) && !value.length)) return 'Not recorded';
  if (Array.isArray(value)) return value.map(humanValue).join(' · ');
  if (typeof value === 'boolean') return value ? 'Yes' : 'No';
  if (typeof value === 'object') {
    const object = value as Record<string, unknown>;
    if ('value' in object && 'precision' in object) {
      return `${String(object.value)} (${String(object.precision)} precision)`;
    }
    if ('recordId' in object && 'revision' in object) {
      return `Another record · revision ${String(object.revision)}`;
    }
    return Object.entries(object)
      .map(([key, item]) => `${fieldLabel(key)}: ${humanValue(item)}`)
      .join(' · ');
  }
  const string = String(value);
  return /^https?:\/\//i.test(string) ? shortUrl(string) : string;
}

export function formatTimestamp(value?: string): string {
  if (!value) return '—';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const sameYear = date.getFullYear() === new Date().getFullYear();
  return date.toLocaleString([], {
    ...(sameYear ? {} : { year: 'numeric' }),
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  });
}

export function timeAgo(value?: string): string {
  if (!value) return '—';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const seconds = Math.round((Date.now() - date.getTime()) / 1000);
  if (seconds < 60) return 'just now';
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.round(hours / 24);
  if (days < 30) return `${days} d ago`;
  return formatTimestamp(value);
}

/**
 * Resolves a state label and its colour tone for the shared StatePill component.
 */
export function statePillLabel(kind: string, value: string): string {
  const maps: Record<string, Record<string, string>> = {
    job: JOB_STATE_LABELS,
    run: RUN_STATE_LABELS,
    analysis: ANALYSIS_STATE_LABELS,
    health: HEALTH_STATE_LABELS,
    verification: VERIFICATION_LABELS,
    reviewState: REVIEW_STATE_LABELS,
    disposition: DISPOSITION_LABELS,
    track: LOCAL_TRACK_STATE_LABELS,
    migration: MIGRATION_STATUS_LABELS
  };
  return maps[kind]?.[value] ?? value;
}

export function stateTone(kind: string, value: string): string {
  if (kind === 'health') {
    if (value === 'ready') return 'ok';
    if (value === 'limited') return 'warn';
    return 'danger';
  }
  if (kind === 'job') {
    if (value === 'done') return 'ok';
    if (value === 'error' || value === 'cancelled') return 'danger';
    if (value === 'review') return 'info';
    return 'warn';
  }
  if (kind === 'run' || kind === 'analysis') {
    if (value === 'completed' || value === 'done') return 'ok';
    if (value === 'failed' || value === 'error' || value === 'interrupted') return 'danger';
    return 'info';
  }
  if (kind === 'disposition') {
    if (value === 'selected') return 'ok';
    if (value === 'corroborated') return 'info';
    if (value === 'rejected') return 'danger';
    return 'warn';
  }
  if (kind === 'verification') {
    if (value === 'curator-confirmed') return 'ok';
    if (value === 'source-confirmed') return 'info';
    return 'warn';
  }
  if (kind === 'reviewState') return value === 'review' ? 'warn' : 'ok';
  if (kind === 'track') {
    if (value === 'written' || value === 'enriched') return 'ok';
    if (value === 'error') return 'danger';
    if (value === 'skipped') return 'warn';
    return 'neutral';
  }
  if (kind === 'migration') {
    if (value === 'imported' || value === 'existing') return 'ok';
    if (value === 'partial') return 'warn';
    return 'neutral';
  }
  return 'neutral';
}


