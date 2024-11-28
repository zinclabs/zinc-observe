import path from 'path';
import fs from 'fs/promises';
import { SnapshotLocation } from './SnapshotLocation';
import { ENABLED_LOGGING } from '../constant/const';

const originalLog = console.log;

// Override console.log
console.log = (...args: any[]) => {
  // Custom logic before logging
  if(ENABLED_LOGGING){
    originalLog(...args);
  }
    // Add your custom prefix or logic
};

export class LocalSnapshotLocation implements SnapshotLocation {
  private localFolderPath: string;
  private version: string;

  constructor(version: string) {
    this.localFolderPath =  path.join(process.cwd(), "tests");
    this.version = version;
  }

  // List all snapshots (files and folders) in a given version folder
  async listSnapshots(folderPath:""): Promise<string[]> {
    console.log("LocalSnapshot:listSnapshots- localFolderPath", this.localFolderPath)
    const versionFolder = path.join(this.localFolderPath,this.version, folderPath);
    try {
      console.log("LocalSnapshot:listSnapshots- versionFolder", versionFolder)
      const items = await fs.readdir(versionFolder);
      console.log("LocalSnapshot:listSnapshots- items", items)
      return items;
    } catch (error) {
      throw new Error(`Error listing snapshots in version ${this.version}: ${error}`);
    }
  }

  // Fetch a specific snapshot (file) from the local folder
  async fetchSnapshot(snapshotKey: string): Promise<Buffer> {
    const filePath = path.join(this.localFolderPath, this.version, snapshotKey);
    console.log("LocalSnapshot:fetchSnapshot- filePath", filePath)
    try {
      const data = await fs.readFile(filePath);
      console.log("LocalSnapshot:fetchSnapshot- data", data)
      return data;
    } catch (error) {
      throw new Error(`Error fetching snapshot ${snapshotKey} from version ${this.version}: ${error}`);
    }
  }

  // Save a snapshot (file) to the local folder
  async saveSnapshot(snapshotKey: string, data: Buffer): Promise<void> {

    const filePath = path.join(this.localFolderPath, this.version, snapshotKey);
    console.log("LocalSnapshot:saveSnapshot- filePath", filePath)
    try {
      await fs.mkdir(path.dirname(filePath), { recursive: true });
      await fs.writeFile(filePath, data);
    } catch (error) {
      throw new Error(`Error saving snapshot ${snapshotKey} for version ${this.version}: ${error}`);
    }
  }

  }
