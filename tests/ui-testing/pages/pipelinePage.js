const { expect } = require('@playwright/test')
class PipelinePage {
  constructor(page) {
    this.page = page;
    // Locators
    this.pipelineMenuLink = page.locator(
      '[data-test="menu-link-\\/pipeline-item"]'
    );
    this.addPipelineButton = page.locator(
      '[data-test="pipeline-list-add-pipeline-btn"]'
    );
    this.streamButton = page.getByRole("button", { name: "Stream" }).first();
    this.queryButton = page.getByRole("button", { name: "Query" });
    this.vueFlowPane = page.locator(".vue-flow__pane");
    this.logsDropdown = page.locator("div").filter({ hasText: /^logs$/ });
    this.logsOption = page
      .getByRole("option", { name: "logs" })
      .locator("div")
      .nth(2);
    this.saveButton = page.locator('[data-test="input-node-stream-save-btn"]');
    this.selectStreamError = page.getByText("Please select Stream from the");
    this.savePipelineButton = page.locator(
      '[data-test="add-pipeline-save-btn"]'
    );
    this.pipelineNameRequiredMessage = page.getByText(
      "Pipeline name is required"
    );
    this.pipelineNameInput = page.getByLabel("Enter Pipeline Name");
    this.sourceNodeRequiredMessage = page.getByText("Source node is required");
    this.streamNameInput = page.getByLabel("Stream Name *");
    this.e2eAutomateOption = page.getByRole("option", { name: "e2e_automate" });
    this.inputNodeStreamSaveButton = page.locator(
      '[data-test="input-node-stream-save-btn"]'
    );
    this.destinationNodeRequiredMessage = page.getByText(
      "Destination node is required"
    );
    this.deleteButton = page.locator("button").filter({ hasText: "delete" });
    this.confirmDeleteButton = page.locator('[data-test="confirm-button"]');
    this.secondStreamButton = page.getByRole('button', { name: 'Stream' }).nth(1);
    this.functionButton =  page.getByRole('button', { name: 'Function' })
    this.conditionButton =  page.getByRole('button', { name: 'Condition' })
   this.selectPreviousNodeDropdown = page.getByLabel('Select Previous Node');
   this.previousNodeDropdown = page.locator('[data-test="previous-node-dropdown-input-stream-node-option"]');
   this.previousNodeDropdownSecond = page.locator('[data-test="previous-node-dropdown-input-stream-node-option"]:last-child');
    this.createStreamToggle = page.locator('[data-test="create-stream-toggle"] div').nth(2);
    this.saveStreamButton = page.locator('[data-test="save-stream-btn"]');
    this.inputNodeStreamSaveButton = page.locator('[data-test="input-node-stream-save-btn"]')
    this.pipelineSearchInput = page.locator('[data-test="pipeline-list-search-input"]');
    this.confirmDeleteButton = page.locator('[data-test="confirm-button"]');
    this.deletionSuccessMessage = page.getByText('Pipeline deleted successfully')
    this.sqlEditor = page.locator('[data-test="scheduled-pipeline-sql-editor"]');
    this.sqlQueryInput = page.locator('.view-lines');
    this.frequencyUnit = page.locator('[data-test="scheduled-pipeline-frequency-unit"]');
    this.saveQueryButton = page.locator('[data-test="stream-routing-query-save-btn"]');
    this.createFunctionToggle = page.locator('[data-test="create-function-toggle"] div').nth(2);
    this.functionNameLabel = page.locator('[data-test="add-function-node-routing-section"]').getByLabel('Name');
    this.associateFunctionSaveButton = page.locator('[data-test="associate-function-save-btn"]');
    this.associateNewFunctionSaveButton = page.locator('[data-test="add-function-save-btn"]');
    this.functionNameRequiredError = page.getByText('Function Name is required')
    this.functionRequiredError = page.getByText('Function is required')
    this.streamSelectionError = page.getByText('Please select Stream from the')
    this.columnInput = page.getByPlaceholder('Column'); // Locator for the "Column" input field
    this.columnOption = page.getByRole('option', { name: 'kubernetes_container_name' })
    this.fieldRequiredError = page.getByText('Field is required!')
    this.tableRowsLocator = page.locator("tbody tr");
    this.confirmButton = page.locator('[data-test="confirm-button"]');
    this.functionNameInput = page.locator('[data-test="add-function-name-input"]');
    this.addConditionSaveButton = page.locator('[data-test="add-condition-save-btn"]');
    this.pipelineMenu = '[data-test="menu-link-\\/pipeline-item"]';
    this.enrichmentTableTab =
      '[data-test="function-enrichment-table-tab"] > .q-tab__content > .q-tab__label';
    this.addEnrichmentTableButton = 'text=Add Enrichment Table';



  }

  async openPipelineMenu() {
    await this.pipelineMenuLink.click();
  }

  async addPipeline() {
    await this.addPipelineButton.waitFor({ state: 'visible' });
    await this.addPipelineButton.click();
  }

  async selectStream() {
    await this.streamButton.click();
  }

  async dragStreamToTarget(streamElement, offset = { x: 0, y: 0 }) {
    const targetBox = await this.vueFlowPane.boundingBox();
    const streamBox = await streamElement.boundingBox();
    targetBox.x += offset.x;
  targetBox.y += offset.y;
    if (streamBox && targetBox) {
      await this.page.mouse.move(streamBox.x + streamBox.width / 2, streamBox.y + streamBox.height / 2);
      await this.page.mouse.down();
      await this.page.mouse.move(targetBox.x + targetBox.width / 2, targetBox.y + targetBox.height / 2);
      await this.page.mouse.up();
    }
  }

  async selectLogs() {
    await this.logsDropdown.click();
    await this.logsOption.click();
  }

  async saveStream() {
    await this.saveButton.click();
  }

  async confirmStreamError() {
    await this.selectStreamError.click();
  }

  async savePipeline() {
    await this.savePipelineButton.click();
  }

  async confirmPipelineNameRequired() {
    await this.pipelineNameRequiredMessage.click();
  }

  async enterPipelineName(pipelineName) {
    await this.pipelineNameInput.click();
    await this.pipelineNameInput.fill(pipelineName);
  }

  async confirmSourceNodeRequired() {
    await this.sourceNodeRequiredMessage.click();
  }
  async enterStreamName(streamName) {
    await this.streamNameInput.fill(streamName);
    await this.streamNameInput.press("Enter");
  }

  async selectStreamOption() {
    await this.e2eAutomateOption.click();
  }

  async saveInputNodeStream() {
    await this.inputNodeStreamSaveButton.click();
  }

  async confirmDestinationNodeRequired() {
    await this.destinationNodeRequiredMessage.click();
  }
  async deletePipeline() {
    await this.deleteButton.click();
  }

  async confirmDelete() {
    await this.confirmDeleteButton.click();
  }

  async selectAndDragSecondStream() {
    await this.secondStreamButton.click();
    await this.dragStreamToTarget(this.secondStreamButton,{ x: 80, y: 80 });
  }


  async selectAndDragFunction() {
    await this.secondStreamButton.click();
    await this.dragStreamToTarget(this.functionButton, { x: 50, y: 50 });
  }


  async selectPreviousNode() {
    await this.selectPreviousNodeDropdown.click();
  }

  async selectPreviousNodeDrop() {
    await this.previousNodeDropdownSecond.click();
  }

  // Method to toggle the create stream option
  async toggleCreateStream() {
    await this.createStreamToggle.click();
  }

  async clickSaveStream() {
    await this.saveStreamButton.click();
  }

  // Method to click the input node stream save button
  async clickInputNodeStreamSave() {
    await this.inputNodeStreamSaveButton.click();
  }
  async searchPipeline(pipelineName) {
    await this.pipelineSearchInput.click();
    await this.pipelineSearchInput.fill(pipelineName);
  }

  async confirmDeletePipeline() {
    await this.confirmDeleteButton.click();
  }

  // Method to verify deletion success message
  async verifyPipelineDeleted() {
    await this.deletionSuccessMessage.click();
  }


  async selectStream() {
    await this.queryButton.click();
  }
  async clickSqlEditor() {
    await this.sqlEditor.click();
  }

  // Method to type the SQL query
  async typeSqlQuery(query) {
    await this.sqlQueryInput.click();
    await this.page.keyboard.type(query);
  }

  // Method to select the frequency unit dropdown
  async selectFrequencyUnit() {
    await this.frequencyUnit.click();
  }

  // Method to save the query
  async saveQuery() {
    await this.saveQueryButton.click();
  }


  async selectFunction() {
    await this.functionButton.click();
  }

  async toggleCreateFunction() {
    await this.createFunctionToggle.click();
}

async clickFunctionLabel(name) {
  // Click the label and then fill in the input that appears
  await this.functionNameLabel.click();
}
async saveFunction() {
  await this.associateFunctionSaveButton.click();
}

async saveNewFunction() {
  await this.associateNewFunctionSaveButton.click();
}

async enterFunctionName(name) {
  await this.functionNameInput.fill(name);
}

async assertFunctionNameRequiredErrorVisible() {
  await expect(this.functionNameRequiredError).toBeVisible();
}

async assertFunctionNameRequiredErrorNotToBeVisible() {
  await expect(this.functionNameRequiredError).not.toBeVisible();
}

async assertFunctionRequiredErrorVisible() {
  await expect(this.functionRequiredError).toBeVisible();
}
async assertStreamSelectionErrorVisible() {
  await expect(this.streamSelectionError).toBeVisible();
}

async selectAndDragCondition() {
  await this.dragStreamToTarget(this.conditionButton, { x: 50, y: 50 });
}
async fillColumnAndSelectOption(columnName) {
  await this.columnInput.click();
  await this.columnInput.fill(columnName);
  await this.columnOption.click();
}
async saveCondition() {
  await this.addConditionSaveButton.click();
}
async verifyFieldRequiredError() {
  await this.fieldRequiredError.waitFor({ state: 'visible' });
  await this.fieldRequiredError.click();
}

async navigateToAddEnrichmentTable() {
  await this.page.locator(this.pipelineMenu).click();
  await this.page.click(this.enrichmentTableTab, { force: true });
  await this.page.getByText("Add Enrichment Table").click();
}

async uploadEnrichmentTable(fileName, fileContentPath) {
  // Set the file to be uploaded
  const inputFile = await this.page.locator(this.fileInput);
  await inputFile.setInputFiles(fileContentPath);

  // Enter the file name
  await this.page.fill(this.fileNameInput, fileName);

  // Click on 'Save'
  await this.page.getByText(this.saveButton).click({ force: true });

  // Wait for the process to complete
  await this.page.reload();
  await this.page.waitForTimeout(3000);

  // Search for the uploaded file
  await this.page.getByPlaceholder(this.searchPlaceholder).fill(fileName);
  await this.page.waitForTimeout(3000);
}
async deleteEnrichmentTableByName(fileName) {
  const rows = await this.tableRowsLocator;
  let fileFound = false;

  for (let i = 0; i < (await rows.count()); i++) {
    const row = rows.nth(i);
    const functionName = await row
      .locator("td.text-left")
      .nth(1)
      .textContent();

    if (functionName?.trim() === fileName) {
      fileFound = true;
      console.log("Uploaded file found:", functionName);

      // Click the 'Delete Function' button
      await row.locator('[title="Delete Function"]').click();

      // Confirm the deletion
      await this.confirmButton.click();
      break;
    }
  }

  if (!fileFound) {
    throw new Error(
      `Uploaded file "${fileName}" not found in the enrichment table.`
    );
  }

}

async navigateToEnrichmentTableTab() {
  await this.pipelineMenuLink.click();
  await this.enrichmentTableTab.click();
}

}

export default PipelinePage;