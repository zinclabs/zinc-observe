self.onmessage = (event) => {
    const { file, chunkSize, fileName, enrichmentTableName } = event.data;
  
    const totalChunks = Math.ceil(file.size / chunkSize);
    let uploadedBytes = 0;
    let uploadedChunks = 0;
  
    const uploadChunk = (chunkIndex) => {
      // Simulate chunk processing
      const start = chunkIndex * chunkSize;
      const end = Math.min(start + chunkSize, file.size);
      const chunk = file.slice(start, end);
  
      setTimeout(() => {
        uploadedChunks++;
        uploadedBytes += chunk.size;

        // Calculate progress percentage
      const progress = Math.min((uploadedBytes / file.size) * 100, 100).toFixed(2);

        self.postMessage({
          status: "chunk-uploaded",
          chunk: chunkIndex + 1,
          progress: Number(progress),
        });
  
        if (uploadedChunks < totalChunks) {
          uploadChunk(chunkIndex + 1);
        } else {
          self.postMessage({
            status: "upload-complete",
          });
        }
      }, 500); // Simulate network latency
    };
  
    // Start processing the first chunk
    uploadChunk(0);
  };
      