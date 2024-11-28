import { ENABLED_LOGGING , process } from '../constant/const';
import { SnapshotLocation } from './SnapshotLocation';
import { S3Client, GetObjectCommand, PutObjectCommand, ListObjectsCommand, CopyObjectCommand, HeadObjectCommand, ListBucketsCommand, DeleteObjectsCommand } from '@aws-sdk/client-s3';

const originalLog = console.log;

// Override console.log
console.log = (...args: any[]) => {
  // Custom logic before logging
  if(ENABLED_LOGGING){
    originalLog(...args);
  }
    // Add your custom prefix or logic
};

export class RemoteSnapshotLocation implements SnapshotLocation {
  private s3: S3Client;
  private bucket: string;
  private version: string;

  constructor(bucket: string, basePath: string, region: string, version: string) {
    this.s3 = new S3Client({
      region: region,
      credentials: {
        accessKeyId: process.env.ACCESS_KEY_ID,
        secretAccessKey: process.env.SECRET_ACCESS_KEY,
      },
      endpoint: basePath,
    });

    this.bucket = bucket;
    this.version = version;
  }


  async fetchSnapshot(fileName: string): Promise<Buffer> {
    console.log("RemoteSnapshot:fetchSnapshot- fileName", fileName)
    const key = `snapshots/${this.version}/${fileName}`;
    console.log("RemoteSnapshot:fetchSnapshot- key", key)
  
    const command = new GetObjectCommand({
      Bucket: this.bucket,
      Key: key,
    });
    const response = await this.s3.send(command);
    console.log("RemoteSnapshot:fetchSnapshot- response", response)

    if (response.Body) {
      const chunks: Uint8Array[] = [];
      for await (const chunk of response.Body as any) {
        chunks.push(chunk);
      }
      console.log("RemoteSnapshot:fetchSnapshot- chunks", chunks)
      return Buffer.concat(chunks);
    }

    console.log("RemoteSnapshot:fetchSnapshot- response.Body", response.Body)
    return Buffer.from('');

  }

  async listSnapshots(folderPath: string = ''): Promise<string[]> {
    console.log("RemoteSnapshot:listSnapshots- folderPath", folderPath)

    const basePrefix = `snapshots/${this.version}/`;
    const prefix = folderPath ? `${basePrefix}${folderPath}/` : basePrefix;
  
    console.log("RemoteSnapshot:listSnapshots- basePrefix", basePrefix)
    console.log("RemoteSnapshot:listSnapshots- prefix", prefix)
  
    // Initialize an empty array to store the keys
    const items: string[] = [];
    let marker: string | undefined = undefined;
  
    // Loop until all keys are fetched
    do {
      const command:any = new ListObjectsCommand({
        Bucket: this.bucket,
        Prefix: prefix,
        Marker: marker, // Use marker for pagination
      });
  
      const response:any = await this.s3.send(command);
      console.log("RemoteSnapshot:listSnapshots- response", response)
  
      // Extract keys from the response
      if (response.Contents) {
        response.Contents.forEach((obj: any) => {
          if (obj.Key) {
            items.push(obj.Key.replace(basePrefix, '')); // Add the file key
          }
        });
      }
  
      console.log("RemoteSnapshot:listSnapshots- items", items)
  
      // Update marker for pagination
      marker = response.Contents && response.Contents.length > 0
        ? response.Contents[response.Contents.length - 1].Key
        : undefined;
  
      console.log("RemoteSnapshot:listSnapshots- marker", marker)
    } while (marker); // Continue until all keys are fetched
  
    console.log("RemoteSnapshot:listSnapshots- items", items)
    return items;
  }
  

  async saveSnapshot(name: string, data: Buffer): Promise<void> {
    console.log("RemoteSnapshot:saveSnapshot- name", name)

    const key = `snapshots/${this.version}/${name}`;
    const command = new PutObjectCommand({
      Bucket: this.bucket,
      Key: key,
      Body: data,
    });
    console.log("RemoteSnapshot:saveSnapshot- command", command)
    await this.s3.send(command);
  }

  async checkVersionExists(): Promise<boolean> {
    console.log("RemoteSnapshot:checkVersionExists- this.version", this.version)
    const versionKey = `snapshots/${this.version}/`;

    console.log("RemoteSnapshot:checkVersionExists- versionKey", versionKey)
    const command = new ListObjectsCommand({
      Bucket: this.bucket,
      Prefix: versionKey,
      MaxKeys: 1, // We only need to check if at least one object exists
    });

    try {
      const response = await this.s3.send(command);

      console.log("RemoteSnapshot:checkVersionExists- response", response)
  
      // Check if there are any objects under the given prefix
      return !!(response.Contents && response.Contents.length > 0);
    } catch (error) {
      console.error(`Error checking version existence: ${(error as Error).message}`);
      throw error;
    }
  }

  async backupVersion(): Promise<void> {
    console.log("RemoteSnapshot:backupVersion- this.version", this.version)

    const sourcePrefix = `snapshots/${this.version}/`;
    const backupPrefix = `snapshots/${this.version}_backup_${Date.now()}/`;

    console.log("RemoteSnapshot:backupVersion- sourcePrefix", sourcePrefix)
    console.log("RemoteSnapshot:backupVersion- backupPrefix", backupPrefix)
  
    const command = new ListObjectsCommand({
      Bucket: this.bucket,
      Prefix: sourcePrefix,
    });
  
    try {
      const response = await this.s3.send(command);
  
      console.log("RemoteSnapshot:backupVersion- response", response)
  
      if (response.Contents && response.Contents.length > 0) {
        console.log(`RemoteSnapshot:backupVersion- Backing up version ${this.version}...`);

        const copyPromises = response.Contents.map(async (item) => {
          if (item.Key) {
            console.log(`RemoteSnapshot:backupVersion- Backing up: ${item.Key}`);
            const sourceKey = item.Key;
            const backupKey = sourceKey.replace(sourcePrefix, backupPrefix);
  
            // Handle folders (keys ending with '/')
            if (sourceKey.endsWith('/')) {
              console.log(`RemoteSnapshot:backupVersion- Skipping folder: ${sourceKey}`);
              return; // Skip folders as they are handled implicitly by S3
            }
  
            console.log("RemoteSnapshot:backupVersion- before copy");
            // Copy files to the backup location
            console.log(`RemoteSnapshot:backupVersion- Copying from: ${this.bucket}/${sourceKey} to: ${backupKey}`);
            const encodedCopySource = encodeURIComponent(`${this.bucket}/${sourceKey}`);
  
            const copyCommand = new CopyObjectCommand({
              Bucket: this.bucket,
              CopySource: encodedCopySource,
              Key: backupKey,
            });
  
            try {
              await this.s3.send(copyCommand);
              console.log(`RemoteSnapshot:backupVersion- Backed up: ${sourceKey} to ${backupKey}`);
            } catch (copyError) {
              console.error(`RemoteSnapshot:backupVersion- Error copying ${sourceKey} to ${backupKey}:`, copyError);
              throw copyError;
            }
          }
        });
  
        // Wait for all files to be copied
        await Promise.all(copyPromises);
  
        // After backup, delete the old version
        console.log(`RemoteSnapshot:backupVersion-Deleting old version ${this.version}...`);
  
        // List all objects under the version prefix to be deleted
        const deleteCommand = new DeleteObjectsCommand({
          Bucket: this.bucket,
          Delete: {
            Objects: response.Contents.map(item => ({ Key: item.Key! }))
          }
        });
  
        try {
          const deleteResponse = await this.s3.send(deleteCommand);
          console.log(`RemoteSnapshot:backupVersion- Successfully deleted old version ${this.version}.`);
        } catch (deleteError) {
          console.error(`RemoteSnapshot:backupVersion- Error deleting old version ${this.version}:`, deleteError);
          throw deleteError;
        }
      } else {
        console.log(`RemoteSnapshot:backupVersion- No items found to backup for version ${this.version}.`);
      }
    } catch (error) {
      console.error(`RemoteSnapshot:backupVersion- Error backing up version: ${(error as Error).message}`);
      throw error;
    }
  }
}