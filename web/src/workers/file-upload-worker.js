self.onmessage = (e) => {
  const { file, chunkSize, fileName, orgIdentifier, append } = e.data;
  const totalChunks = Math.ceil(file.size / chunkSize);
  let currentChunk = 0;

  const uploadChunk = (chunk, index) => {
    const formData = new FormData();
    formData.append('file_chunk', chunk);
    formData.append('file_name', fileName);
    formData.append('chunk_index', index);
    formData.append('total_chunks', totalChunks);
    formData.append('file_size', file.size);
    formData.append('append', append);

    // API call for chunk upload
    fetch(`/api/${orgIdentifier}/enrichment_tables/${fileName}?append=${append}`, {
      method: 'POST',
      body: formData,
    })
      .then((response) => response.json())
      .then((data) => {
        postMessage({ status: 'chunk-uploaded', chunk: index });
        if (currentChunk < totalChunks - 1) {
          uploadNextChunk();
        } else {
          postMessage({ status: 'upload-complete' });
        }
      })
      .catch((err) => {
        postMessage({ status: 'error', error: err.message });
      });
  };

  const uploadNextChunk = () => {
    const start = currentChunk * chunkSize;
    const end = Math.min(start + chunkSize, file.size);
    const chunk = file.slice(start, end);
    uploadChunk(chunk, currentChunk);
    currentChunk++;
  };

  uploadNextChunk(); // Start uploading the first chunk
};
