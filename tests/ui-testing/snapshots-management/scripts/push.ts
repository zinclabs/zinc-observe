import { readFileSync, statSync, readdirSync, lstatSync } from "fs";
import path from "path";
import inquirer from "inquirer";
import { RemoteSnapshotLocation } from "../snapshot-locations/RemoteSnapshotLocation";
import config from "../snapshots.config.json";
import { ENABLED_LOGGING } from "../constant/const";

const originalLog = console.log;

// custom isEmpty to check if an array is empty or not
function isEmpty(array: string[]){
  return true ? array.length == 0 : false;
}

// Override console.log
console.log = (...args: any[]) => {
  if (ENABLED_LOGGING) {
    console.log(...args);
  }
};

async function pushSnapshots(): Promise<void> {
  try {
    const snapshotLocation = new RemoteSnapshotLocation(
      config.bucket,
      config.basePath,
      config.region,
      config.currentVersion
    );
    const version = config.currentVersion;
    const localFolderPath = path.join(process.cwd(), "playwright-tests", "__snapshots__");

    console.log(`Pushscript: Checking if the local folder exists: ${localFolderPath}...`);
    if (!lstatSync(localFolderPath).isDirectory()) {
      console.error(`The folder ${localFolderPath} does not exist.`);
      process.exit(1);
    }

    const remoteFiles = await snapshotLocation.fetchFiles(); // fetch snapshots from remote
    const localFiles = getFilesInDirectory(localFolderPath); // fetch local snapshots using custom function

    console.log("Pushscript: Comparing local files with remote files...");
    const newFilesList = newFiles(remoteFiles, localFiles); // diff between local and remote
    const overwriteFiles = findFilesWithSameNames(remoteFiles, localFiles); // same files in both local and remote
    const removedFiles = newFiles(localFiles, remoteFiles); // diff between remote and local

    console.log("Pushscript: The following snapshots are new and not in remote:");
    console.log(`Pushscript: `,newFilesList.map(file => file));

    console.log("Pushscript: The following snapshots have the same name in remote:");
    console.log(`Pushscript: `,overwriteFiles.map(file => file));

    console.log("Pushscript: The following snapshots are removed from local but are present in remote:");
    console.log(`Pushscript: `,removedFiles.map(file => file));

    console.log(`Pushscript: Checking if version ${version} exists in the storage...`);
    const versionExists = await snapshotLocation.checkVersionExists(); // check if the specified version exists on remote
    console.log(`Pushscript: Version ${version} found ${versionExists}`)

    // if there are only new snapshots incoming (there are no existing snapshots) and if the version exists so ask for backup
    if (isEmpty(overwriteFiles) && versionExists) { 
      const { action } = await inquirer.prompt([
        {
          type: "list",
          name: "action",
          message: `The version ${version} already exists. Overwriting will create a backup. Do you wish to continue?`,
          choices: [
            { name: "Proceed", value: "proceed" },
            { name: "Cancel", value: "cancel" },
          ],
        },
      ]);
      if (action === "cancel") {
        console.log("Pushscript: Process aborted by the user.");
        return;
      }
    
      console.log(`Pushscript: Creating a backup for version ${version}...`);
      await snapshotLocation.backupVersion();
      await uploadFiles(localFiles, snapshotLocation, localFolderPath);
      return;
    }else if(isEmpty(overwriteFiles) && !versionExists){ // there are no pre-existing snapshots also the version dont exist so push all snapshots
      console.log("Pushscript: Version do not exist. Pushing all the snapshots on remote.")
      await uploadFiles(localFiles, snapshotLocation, localFolderPath);
      return;
    }else if(!isEmpty(overwriteFiles) && !isEmpty(newFilesList)){ // some existing snapshots as well as some new snapshots
      // prompt the tester to whether upload new files or oveerwrite the existing ones
      const { action } = await inquirer.prompt([
        {
          type: "list",
          name: "action",
          message: "Some existing snapshots and some new snapshots found.",
          choices: [
            "Push the new snapshots in the same version.",
            "Overwrite the existing snapshots and add new snapshots in another version.",
            "Cancel",
          ],
        },
      ]);
      // push new snapshots in the same version
      if (action === "Push the new snapshots in the same version.") {
        await uploadFiles(newFilesList, snapshotLocation, localFolderPath);
        return;
      }
      // overwrite existing ones and add new ones in the different versions or force push
      if (action === "Overwrite the snapshots and add new snapshots in another version.") {
        console.log("Pushscript: Please mention the new version in the config file and continue...")
      }
      // exit from the terminal
      if (action === "Cancel") {
        console.log("Pushscript: Process aborted by the user.");
      }
    }else if((!isEmpty(overwriteFiles) && !isEmpty(newFilesList)) || !isEmpty(removedFiles)){ // (some pre-existing snapshots and no new snapshots) or there are some snapshots removed
      if(!isEmpty(removedFiles)){
        console.log("Pushscript: Some snapshots seems to be removed from local.")

        // prompts the tester for version change or backup version creation 
        const { action } = await inquirer.prompt([  
          {
            type: "list",
            name: "action",
            message: `Do you want to change the version in the config file?`,
            choices: [
              { name: "Proceed", value: "proceed" },
              { name: "Cancel (we'll create a backup version for you)", value: "cancel" },
            ],
          },
        ]);
        if (action === "proceed") {
          console.log("Pushscript: Process aborted by the user.");
          return;
        }
        if (action === "cancel") {
          console.log("Pushscript: Proceeding to create a backup version.");
        }
      }else{
        console.log("Pushscript: Some snapshots seems to be overwritten.")
      }
      console.log("Pushscript: Specify a new version in the config file to continue..")
    }

    // prompts user for backup 
    const { action } = await inquirer.prompt([
      {
        type: "list",
        name: "action",
        message: `If you still wish to continue we'll create a backup for you. Do you wish to continue?`,
        choices: [
          { name: "Proceed", value: "proceed" },
          { name: "Cancel", value: "cancel" },
        ],
      },
    ]);
  
    if (action === "cancel") {
      console.log("Pushscript: Process aborted by the user.");
      return;
    }
  
    console.log(`Pushscript: Creating a backup for version ${version}...`);
    await snapshotLocation.backupVersion();
    await uploadFiles(localFiles, snapshotLocation, localFolderPath);

    console.log(`Pushscript: Pushing new snapshots for version: ${version}`);
    console.log("Pushscript: All snapshots for version have been successfully pushed.");
  } catch (error) {
    console.error(`Error pushing snapshots: ${(error as Error).message}`);
    process.exit(1);
  }
}

// custom function to fetch all the snapshots from the local __snapshots__ directory
function getFilesInDirectory(directory: string, parentPath: string = ""): string[] {
  let fileList: string[] = [];
  const items = readdirSync(directory);

  for (const item of items) {
    const fullPath = path.join(directory, item);
    const stats = statSync(fullPath);

    if (stats.isDirectory()) {
      fileList = fileList.concat(getFilesInDirectory(fullPath, path.join(parentPath, item)));
    } else {
      fileList.push(`${parentPath}/${item}`);
    }
  }
  return fileList;
  // return the name in the format:-    testcase_name/snapshot_name
}

// custom function to find out diff between the two storages
// it can either be diff between local and remote or remote and local
function newFiles(remoteFiles: string[], localFiles: string[]): string[] {
  const result: string[] = [];
  const remoteFileSet = new Set<string>();
  remoteFiles.forEach(file => remoteFileSet.add(file));

  localFiles.forEach(localFile => {
    if (!remoteFileSet.has(localFile)) {
      result.push(localFile);
    }
  });

  return result;
}

// custom function to find out the same or existing files in both storages (local and s3 bucket)
function findFilesWithSameNames(remoteFiles: string[], localFiles: string[]): string[] {
  const result: string[] = [];
  const remoteFileSet = new Set<string>();
  remoteFiles.forEach(file => remoteFileSet.add(file));

  localFiles.forEach(localFile => {
    if (remoteFileSet.has(localFile)) {
      result.push(localFile);
    }
  });

  return result;
}

// custom function to upload files on the s3 bucket 
async function uploadFiles(files: string[], snapshotLocation: RemoteSnapshotLocation, basePath: string): Promise<void> {
  for (const file of files) {
    const filePath = path.join(basePath, file);
    const remotePath = file.replace(/\\/g, "/");
    const data = readFileSync(filePath);

    try {
      await snapshotLocation.saveSnapshot(remotePath, data);
      console.log(`Pushscript: Successfully uploaded: ${remotePath}`);
    } catch (error) {
      console.error(`Failed to upload ${remotePath}: ${(error as Error).message}`);
    }
  }
}

pushSnapshots();
