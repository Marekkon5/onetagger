import { test, expect, type Page } from '@playwright/test';

async function login(page: Page) {
  await page.goto('/login');
  await page.getByLabel('Curator password').fill('local-e2e-curator-12345');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.getByRole('button', { name: 'CURATOR', exact: true })).toBeVisible();
}

test.beforeEach(async ({ page }) => {
  // Provider discovery is an external boundary. Catalog writes/auth/reads below use the real SQLite API.
  await page.route('**/api/providers', route => route.fulfill({ json: ['freeteknomusic', 'archiveorg', 'soundcloud', 'discogs', 'youtube', 'hearthis', 'mixcloud'].map(id => ({ id, state: id === 'soundcloud' ? 'limited' : 'ready', detail: 'Deterministic provider boundary for click tests', checkedAt: new Date().toISOString() })) }));
});

test('public readers click all five indexes and shared detail links', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  await expect(page).toHaveTitle(/Mixsets/i);
  await expect(page.getByRole('heading', { name: 'Mixes', exact: true })).toBeVisible();
  await page.getByRole('link', { name: /Shared Click Recording/ }).click();
  await expect(page.getByRole('heading', { name: 'Shared Click Recording' })).toBeVisible();
  await expect(page.getByTestId('catalog-editor')).toHaveCount(0);
  await page.getByRole('link', { name: /DJ Live/ }).click();
  await expect(page.getByRole('heading', { name: 'DJ Live', exact: true })).toBeVisible();
  await expect(page.getByText('Fixture Performer', { exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: /Second Shared Recording/ })).toBeVisible();
  for (const [path, title, row] of [['/artists', 'Artists', 'DJ Live'], ['/crews', 'Crews', 'Click Test Crew'], ['/labels', 'Labels', 'Click Test Label'], ['/events', 'Events', 'Click Test Event']]) {
    await page.goto(path);
    await expect(page.getByRole('heading', { name: title, exact: true })).toBeVisible();
    await page.getByRole('link', { name: new RegExp(row) }).click();
    await expect(page.getByRole('heading', { name: row, exact: true })).toBeVisible();
  }
  await page.screenshot({ path: '/tmp/syco23-catalogue-verification/public-event-desktop.png', fullPage: true });
  expect(errors).toEqual([]);
});

test('event-only mix shows linked event without a performer', async ({ page }) => {
  await page.goto('/mix/mix_e2e_event');
  await expect(page.getByRole('heading', { name: 'Event Only Recording' })).toBeVisible();
  await page.getByRole('link', { name: /Click Test Event/ }).click();
  await expect(page.getByRole('heading', { name: 'Click Test Event', exact: true })).toBeVisible();
});

test('curator edits typed fields and a fresh public browser sees committed values and evidence', async ({ page, browser }) => {
  await login(page);
  await page.goto('/entity/crew_e2e_001');
  await expect(page.getByTestId('catalog-editor')).toBeVisible();
  await page.getByLabel('Profile', { exact: true }).fill('Curated crew profile from click test');
  await page.getByTestId('save-record').click();
  await expect(page.locator('dl').getByText('Curated crew profile from click test', { exact: true }).first()).toBeVisible();
  const context = await browser.newContext();
  const publicPage = await context.newPage();
  await publicPage.goto('http://localhost:15173/entity/crew_e2e_001');
  await expect(publicPage.locator('dl').getByText('Curated crew profile from click test', { exact: true }).first()).toBeVisible();
  await expect(publicPage.getByTestId('catalog-editor')).toHaveCount(0);
  await expect(publicPage.getByTestId('field-evidence').first()).toBeVisible();
  await context.close();
});

test('review rejection remains rejected after reload', async ({ page }) => {
  await login(page);
  await page.goto('/review');
  await expect(page.getByText('Disposable provider conflict proving selected country protection')).toBeVisible();
  await page.getByRole('button', { name: 'Reject', exact: true }).first().click();
  await expect(page.getByText('Review queue is clear.')).toBeVisible();
  await page.reload();
  await expect(page.getByText('Review queue is clear.')).toBeVisible();
  await page.goto('/entity/artist_e2e_001');
  await expect(page.locator('dl').getByText('DE', { exact: true }).first()).toBeVisible();
});

test('search, empty state and narrow navigation respond to clicks without overflow', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/artists');
  await page.getByRole('searchbox', { name: 'Search artists' }).fill('does-not-exist');
  await page.getByRole('button', { name: 'Search', exact: true }).click();
  await expect(page.getByText('No artists match this search.')).toBeVisible();
  await page.getByRole('button', { name: 'Clear search' }).click();
  await expect(page.getByRole('link', { name: /DJ Live/ })).toBeVisible();
  await page.getByRole('link', { name: /DJ Live/ }).click();
  await expect(page.getByRole('heading', { name: 'DJ Live', exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await page.screenshot({ path: '/tmp/syco23-catalogue-verification/artist-mobile.png', fullPage: true });
});

test('discovery import enrichment and shared reload use committed catalog records', async ({ page, browser }) => {
  await login(page);
  await page.goto('/import');
  await page.locator('.provider-switch').getByRole('button', { name: /YouTube/i }).click();
  await page.getByLabel('QUERY OR DIRECTORY URL').fill('https://www.youtube.com/watch?v=e2eImport01');
  await page.getByRole('checkbox', { name: /Auto-enrich/ }).uncheck();
  await page.getByRole('button', { name: 'Queue discovery job' }).click();
  await expect(page.getByRole('heading', { name: 'Imported Click Recording', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Add to index', exact: true }).click();
  await page.getByRole('link', { name: 'Open indexed', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Imported Click Recording', exact: true })).toBeVisible();
  const detailUrl = page.url();
  await page.getByRole('button', { name: 'Enrich missing fields' }).click();
  await expect(page.getByTestId('run-history')).toBeVisible();
  await expect(page.getByText(/still missing/).first()).toBeVisible();
  await page.reload();
  await expect(page.getByTestId('run-history')).toBeVisible();
  const publicContext = await browser.newContext();
  const publicPage = await publicContext.newPage();
  await publicPage.goto(detailUrl);
  await expect(publicPage.getByRole('heading', { name: 'Imported Click Recording', exact: true })).toBeVisible();
  await expect(publicPage.getByTestId('catalog-editor')).toHaveCount(0);
  await publicContext.close();
});

test('curator creates an event and preserves month precision through reload', async ({ page }) => {
  await login(page);
  await page.goto('/events');
  await page.getByTestId('create-record').click();
  await page.getByLabel('Event name', { exact: true }).fill('Created Click Event');
  await page.getByLabel('Start date (YYYY, YYYY-MM or YYYY-MM-DD)', { exact: true }).fill('2026-09');
  await page.getByLabel('Venue', { exact: true }).fill('Created venue');
  await page.getByTestId('save-record').click();
  await expect(page.getByRole('heading', { name: 'Created Click Event', exact: true })).toBeVisible();
  await expect(page.locator('dl').getByText('2026-09 (month precision)', { exact: true })).toBeVisible();
  await page.reload();
  await expect(page.locator('dl').getByText('2026-09 (month precision)', { exact: true })).toBeVisible();
});

test('duplicate merge previews both revisions, retains source conflicts and redirects old details', async ({ page }) => {
  await login(page);
  await page.goto('/entity/crew_e2e_001');
  await page.getByLabel('Duplicate record ID', { exact: true }).fill('crew_e2e_002');
  await page.getByRole('button', { name: 'Preview duplicate' }).click();
  await expect(page.getByRole('heading', { name: 'Duplicate Click Crew', exact: true })).toBeVisible();
  await expect(page.getByText(/SURVIVOR \/ REV/)).toBeVisible();
  await expect(page.getByText(/DUPLICATE \/ REV/)).toBeVisible();
  await page.getByTestId('confirm-merge').click();
  await expect(page.getByRole('heading', { name: 'Duplicate Click Crew', exact: true })).toHaveCount(0);
  await page.goto('/entity/crew_e2e_002');
  await expect(page.getByRole('heading', { name: 'Click Test Crew', exact: true })).toBeVisible();
  await page.goto('/review');
  await expect(page.getByText('Alternative crew profile', { exact: true }).first()).toBeVisible();
  const conflict = page.locator('.review-card').filter({ hasText: 'Alternative crew profile' });
  const refresh = conflict.getByRole('button', { name: 'Refresh evidence' });
  if (await refresh.isVisible()) await refresh.click();
  await conflict.getByRole('button', { name: 'Accept field' }).click();
  await expect(conflict).toHaveCount(0);
  await page.goto('/entity/crew_e2e_001');
  await expect(page.locator('dl').getByText('Alternative crew profile', { exact: true }).first()).toBeVisible();
});

test('browser migration retains originals and can be retried against the same source', async ({ page }) => {
  const localLibrary = JSON.stringify([{ id: 'browser-click-legacy', title: 'Migrated Click Recording', sources: [{ provider: 'archiveorg', externalId: 'browser-click-source', url: 'https://archive.org/details/browser-click-source' }], artists: [], crews: [], genres: [], styles: [], artwork: [] }]);
  await page.addInitScript(value => { localStorage.setItem('syco23.mixsets.library', value); }, localLibrary);
  await login(page);
  await expect(page.getByRole('button', { name: 'Migrate this browser’s library' })).toBeVisible();
  await page.getByRole('button', { name: 'Migrate this browser’s library' }).click();
  await expect(page.getByTestId('migration-results')).toContainText(/1 imported|imported.*1/i);
  expect(await page.evaluate(() => localStorage.getItem('syco23.mixsets.library'))).toBe(localLibrary);
  await page.goto('/mix/browser-click-legacy');
  await expect(page.getByRole('heading', { name: 'Migrated Click Recording' })).toBeVisible();
  await page.goto('/login');
  await page.getByRole('button', { name: 'Migrate this browser’s library' }).click();
  await expect(page.getByTestId('migration-results')).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem('syco23.mixsets.library'))).toBe(localLibrary);
});

test('stale frontend save displays conflict and preserves the newer curator edit', async ({ page, context }) => {
  await login(page);
  await page.goto('/event/event_e2e_001');
  const second = await context.newPage();
  await second.goto('/event/event_e2e_001');
  await second.getByLabel('Venue', { exact: true }).fill('Newer concurrent venue');
  await second.getByTestId('save-record').click();
  await expect(second.locator('dl').getByText('Newer concurrent venue', { exact: true })).toBeVisible();
  await page.getByLabel('Venue', { exact: true }).fill('Stale venue must not win');
  await page.getByTestId('save-record').click();
  await expect(page.getByRole('alert').filter({ hasText: /Revision conflict/i })).toBeVisible();
  await page.getByRole('button', { name: 'Reload record', exact: true }).click();
  await expect(page.locator('dl').getByText('Newer concurrent venue', { exact: true })).toBeVisible();
  await expect(page.locator('dl').getByText('Stale venue must not win', { exact: true })).toHaveCount(0);
  await second.close();
});

test('providers screen lists the credential-free Mixcloud source', async ({ page }) => {
  await page.goto('/providers');
  await expect(page.locator('[data-provider="mixcloud"]')).toBeVisible();
  await expect(page.getByText(/search and metadata need no credentials at all/)).toBeVisible();
});
