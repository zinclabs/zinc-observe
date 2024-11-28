import { readFileSync } from 'fs';
import path from 'path';
import { RemoteSnapshotLocation } from './snapshot/RemoteSnapshotLocation';
import config from './snapshots/config.json';
import inquirer from 'inquirer';
import { readdirSync, lstatSync } from 'fs';
import { ENABLED_LOGGING } from './constant/const';

const originalLog = console.log;

// Override console.log
console.log = (...args: any[]) => {
  // Custom logic before logging
  if(ENABLED_LOGGING){
    originalLog(...args);
  }
    // Add your custom prefix or logic
};


async function pushSnapshots(): Promise<void> {
  try {
    const snapshotLocation = new RemoteSnapshotLocation(config.bucket, config.basePath, config.region, config.currentVersion); 
    const version = config.currentVersion;
    const localFolderPath = path.join(process.cwd(), "tests", '__snapshots__')// Path of the folder to push

    console.log(`push-snapshots : Checking if the local folder exists: ${localFolderPath}...`);
    if (!lstatSync(localFolderPath).isDirectory()) {
      console.error(`The folder ${localFolderPath} does not exist.`);
      process.exit(1);
    }

    console.log(`push-snapshots :Checking if version ${version} exists in the storage...`);
    const versionExists = await snapshotLocation.checkVersionExists();
    console.log("push-snapshots :versionExists:",versionExists)

    if (versionExists) {
      const { proceed } = await inquirer.prompt([
        {
          type: 'confirm',
          name: 'proceed',
          message: `The version ${version} already exists. Overwriting will create a backup. Do you wish to continue?`,
          default: false,
        },
      ]);

      if (!proceed) {
        console.log('push-snapshots :Process aborted by the user.');
        return;
      }

      console.log(`push-snapshots :Creating a backup for version ${version}...`);
      await snapshotLocation.backupVersion();
    }

    console.log(`push-snapshots :Pushing new snapshots for version: ${version}`);

    const uploadFiles = async (folderPath: string, relativePath = '') => {
      const items = readdirSync(folderPath);

      for (const item of items) {
        const itemPath = path.join(folderPath, item);
        const remotePath = `${relativePath}${relativePath ? '/' : ''}${item}`.replace(/\\/g, '/');

        if (lstatSync(itemPath).isDirectory()) {
          // If item is a directory, recurse
          await uploadFiles(itemPath, remotePath);
        } else {
          // If item is a file, upload it
          const data = readFileSync(itemPath);
          try {
            await snapshotLocation.saveSnapshot(remotePath, data);
            console.log(`Successfully uploaded: ${remotePath}`);
          } catch (error) {
            console.error(`Failed to upload ${remotePath}: ${(error as Error).message}`);
          }
        }
      }
    };

    await uploadFiles(localFolderPath);

    console.log(`push-snapshots :All files for version ${version} have been successfully pushed.`);
  } catch (error) {
    console.error(`Error pushing snapshots: ${(error as Error).message}`);
    process.exit(1);
  }
}

pushSnapshots();
