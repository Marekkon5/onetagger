import { test, expect, type Page } from '@playwright/test';

// A small byte payload stands in for an MP3; the auto-tag path is driven by the filename,
// so no real audio decoding is required to exercise acceptance and evidence.
const FAKE_MP3 = Array.from({ length: 600 }, (_, index) => index % 251);

async function stubFolderPicker(page: Page) {
  await page.addInitScript((bytes) => {
    const files: Array<[string, number[]]> = [
      ['Kan10 - Live Mackitek Koilisson III 2013.mp3', bytes as number[]],
      ['track01.mp3', bytes as number[]],
    ];
    (window as unknown as { showDirectoryPicker: unknown }).showDirectoryPicker = async () => ({
      kind: 'directory',
      name: 'FixtureFolder',
      async *entries() {
        for (const [name, data] of files) {
          yield [name, {
            kind: 'file',
            name,
            async getFile() { return new File([new Uint8Array(data)], name, { type: 'audio/mpeg' }); },
            async createWritable() { return { async write() {}, async close() {}, async abort() {} }; },
          }];
        }
      },
      async getDirectoryHandle() { throw new Error('the tagged output folder is not exercised here'); },
      async getFileHandle() { throw new Error('the tagged output file is not exercised here'); },
    });
  }, FAKE_MP3);
}

test('auto-tags a good filename and defers a noisy one', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await stubFolderPicker(page);
  await page.goto('/local-tags');
  await page.getByRole('button', { name: /Choose MP3 folder/ }).click();
  await expect(page.getByText('Kan10 - Live Mackitek Koilisson III 2013.mp3')).toBeVisible();

  await page.getByTestId('auto-tag').click();

  // The well-formed filename is parsed into artist/title and accepted at full evidence.
  const good = page.locator('.local-track').filter({ hasText: 'Kan10 - Live Mackitek Koilisson III 2013.mp3' });
  await expect(good.locator('[data-accepted="true"]')).toHaveCount(1);
  await expect(good.locator('input').first()).toHaveValue('Live Mackitek Koilisson III');
  await expect(good.locator('input').nth(1)).toHaveValue('Kan10');

  // The noisy filename stays below the auto-accept threshold and is left for review.
  const noisy = page.locator('.local-track').filter({ hasText: 'track01.mp3' });
  await expect(noisy.locator('[data-accepted="false"]')).toHaveCount(1);
  await expect(page.getByText(/1 AUTO-ACCEPTED/)).toBeVisible();

  expect(errors).toEqual([]);
});
