import { test, expect } from '@playwright/test';

test('Generate and compare snapshot dynamically based on location', async ({ page }) => {
    // Get the snapshot location (local or remote) based on the config

    page.goto('https://example.com');
    const screenshot = await page.screenshot();

    expect(screenshot).toMatchSnapshot('homepage-snapshot.png');
});