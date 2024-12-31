export interface SnapshotLocation {
    fetchSnapshot(testName: string): Promise<Buffer>;
    saveSnapshot(testName: string, data: Buffer): Promise<void>;
    listSnapshots(folderPath:string): Promise<string[]>;
}