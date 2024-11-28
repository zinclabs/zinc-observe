import path from 'path';
import fs from 'fs/promises';
import { SnapshotLocation } from './snapshot/SnapshotLocation';
import { RemoteSnapshotLocation } from './snapshot/RemoteSnapshotLocation';
import config from './snapshots/config.json';
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


async function fetchSnapshots(): Promise<void> {
    console.log('fetch-snapshots : Fetching snapshots...');
    try {
        const snapshotLocation = new RemoteSnapshotLocation(
            config.bucket,
            config.basePath,
            config.region,
            config.currentVersion
        );

        // Local folder path targeting `__screenshots__` within the given localFolderPath
        const screenshotsFolder = path.join(process.cwd(), "tests", "__snapshots__");

        console.log(`fetch-snapshots : Screenshots folder: ${screenshotsFolder}`);
        // console.log(`fetch-snapshots : BaseLocalFolder folder: ${baseLocalFolder}`);

        // Ensure `__screenshots__` folder exists or clear it
        await prepareScreenshotsFolder(screenshotsFolder);

        // Fetch and process snapshots directly
        const snapshots = await snapshotLocation.listSnapshots('');

        console.log('fetch-snapshots : Number of snapshots:', snapshots.length);
        for (const snapshot of snapshots) {
            console.log(`fetch-snapshots : Fetching snapshot: ${snapshot}`);
            await fetchAndSaveItem(snapshot, screenshotsFolder, snapshotLocation);
        }

        console.log('All snapshots have been cloned.');
    } catch (error) {
        console.error(`Error fetching snapshots: ${(error as Error).message}`);
        process.exit(1);
    }
}

/**
 * Ensures the `__screenshots__` folder exists and is emptied.
 */
async function prepareScreenshotsFolder(folderPath: string): Promise<void> {
    try {
        // Check if folder exists
        await fs.access(folderPath);

        // Folder exists; clean it up (delete all contents)
        const files = await fs.readdir(folderPath);
        for (const file of files) {
            await fs.rm(path.join(folderPath, file), { recursive: true, force: true });
        }
        console.log('fetch-snapshots-prepareScreenshotsFolder : Clearing folder...');
    } catch {
        // Folder does not exist; create it
        await fs.mkdir(folderPath, { recursive: true });
        console.log(`fetch-snapshots-prepareScreenshotsFolder- Created folder: ${folderPath}`);
    }
}

/**
 * Fetches and saves a snapshot (file or folder).
 */
async function fetchAndSaveItem(
    item: string,
    localBaseFolder: string,
    snapshotLocation: SnapshotLocation
): Promise<void> {
    const remoteKey = item;
    const normalizedItem = item.replace(/\\/g, '/');
    const isFolder = normalizedItem.endsWith('/'); // Infer folder by trailing slash
    const localPath = path.join(localBaseFolder, normalizedItem);

    if (isFolder) {
        // Create the folder locally
        await fs.mkdir(localPath, { recursive: true });

        // Recursively fetch and process items in the folder
        const folderItems = await snapshotLocation.listSnapshots(remoteKey);
        for (const subItem of folderItems) {
            await fetchAndSaveItem(subItem, localBaseFolder, snapshotLocation);
        }
    } else {
        // Fetch file content using original remote key
        const data = await snapshotLocation.fetchSnapshot(remoteKey);

        // Ensure parent directory exists before saving the file
        await fs.mkdir(path.dirname(localPath), { recursive: true });

        // Save the file locally
        await fs.writeFile(localPath, data);
    }
}

fetchSnapshots();
