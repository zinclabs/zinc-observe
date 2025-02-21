// extractFieldsWorker.ts
const BATCH_SIZE = 1000; // Process 1000 fields at a time

interface StreamData {
  name: string;
  schema: Array<{ name: string }>;
  settings: {
    defined_schema_fields?: string[];
    full_text_search_keys?: string[];
  };
}

interface ExtractFieldsMessage {
  streamName: string;
  schema: Array<{ name: string }>;
  settings: {
    defined_schema_fields?: string[];
    full_text_search_keys?: string[];
  };
  timestampField: string;
  allField: string;
  selectedStream: string[];
  interestingFieldList: string[];
  useUserDefinedSchemas: string;
}

self.onmessage = async (e: MessageEvent<ExtractFieldsMessage>) => {
  const { 
    streamName,
    schema,
    settings,
    timestampField, 
    allField, 
    selectedStream,
    interestingFieldList,
    useUserDefinedSchemas 
  } = e.data;

  try {
    const fields = useUserDefinedSchemas !== "all_fields" && settings?.defined_schema_fields
      ? [timestampField, ...settings.defined_schema_fields, allField]
      : schema.map(obj => obj.name);

    // Process fields in batches
    for (let i = 0; i < fields.length; i += BATCH_SIZE) {
      const batchFields = fields.slice(i, i + BATCH_SIZE);
      const processedBatch = batchFields.map(field => ({
        name: field,
        ftsKey: settings.full_text_search_keys?.includes(field) || false,
        isSchemaField: true,
        group: streamName,
        streams: [streamName],
        showValues: field !== timestampField && field !== allField,
        isInterestingField: interestingFieldList.includes(field)
      }));

      // Send batch results back to main thread
      self.postMessage({
        type: 'batch',
        data: processedBatch,
        progress: Math.min(((i + BATCH_SIZE) / fields.length) * 100, 100)
      });

      // Allow other tasks to run
      await new Promise(resolve => setTimeout(resolve, 0));
    }

    self.postMessage({ type: 'complete' });
  } catch (error) {
    self.postMessage({ 
      type: 'error', 
      error: error instanceof Error ? error.message : 'Unknown error'
    });
  }
};
