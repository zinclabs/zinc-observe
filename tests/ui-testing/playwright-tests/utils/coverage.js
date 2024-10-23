// coverage-helper.js
export async function extractCoverage(page) {
    // Evaluate the window.__coverage__ variable in the browser context
    const coverage = await page.evaluate(() => window.__coverage__);
    
    if (coverage) {
      const fs = require('fs');
      const path = require('path');
  
      // Save the coverage JSON to a file
      fs.writeFileSync('.nyc_output/coverage.json', JSON.stringify(coverage, null, 2));
      console.log('Coverage data saved to coverage.json');
    } else {
      console.warn('No coverage data found.');
    }
  }