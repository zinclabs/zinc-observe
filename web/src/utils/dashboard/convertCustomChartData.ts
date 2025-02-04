// Copyright 2023 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

/**
 * Converts SQL data into a format suitable for rendering a chart.
 *
 * @param {any} panelSchema - the panel schema object
 * @param {any} searchQueryData - the search query data
 * @param {any} store - the store object
 * @return {Object} - the options object for rendering the chart
 */
export const runJavaScriptCode = (panelSchema: any, searchQueryData: any) => {
  return new Promise((resolve, reject) => {
    console.log("Creating iframe for JS execution");

    const iframe = document.createElement("iframe");
    iframe.style.display = "none";
    iframe.setAttribute("sandbox", "allow-scripts");
    document.body.appendChild(iframe);

    const scriptContent = `
      <script>
        console.log("[Iframe] Script execution started.");
        window.onerror = function(message) {
          console.log("[Iframe] Error occurred", message);
          parent.postMessage({ type: 'error', message: message.toString() }, '*');
        };

        window.addEventListener('message', (event) => {
          if (event.data.type === 'execute') {
            try {
              const userCode = event.data.code.trim();
              const data = JSON.parse(event.data.data);

              console.log("[Iframe] Executing user code:", userCode);
              // Remove potential harmful patterns
              const cleanedCode = userCode.replace(/\\/\\*[\\s\\S]*?\\*\\/|\\/\\/.*|--.*/g, '').trim();

              // Execute code directly and expect option to be defined
              const userFunction = new Function('data', cleanedCode + '; return option;');
              const result = userFunction(data);

              console.log("[Iframe] Execution successful. Result:", result);
              parent.postMessage({ type: 'success', result: JSON.stringify(result) }, '*');
            } catch (error) {
              console.error("[Iframe] Error executing code:", error.message);
              parent.postMessage({ type: 'error', message: error.message }, '*');
            }
          }
        });

        console.log("[Iframe] Ready");
      </script>
    `;

    iframe.srcdoc = scriptContent;

    window.addEventListener("message", function handler(event) {
      if (event.source !== iframe.contentWindow) return;

      console.log("Message received from iframe", event.data);

      window.removeEventListener("message", handler);
      setTimeout(() => {
        document.body.removeChild(iframe);
      }, 1000);

      if (event.data.type === "success") {
        panelSchema.customChartResult = JSON.parse(event.data.result);
        resolve(panelSchema.customChartResult);
      } else if (event.data.type === "error") {
        console.error("Error executing code", event.data.message);
        reject(new Error(event.data.message));
      }
    });

    iframe.onload = () => {
      console.log("Iframe loaded, sending message...");

      iframe?.contentWindow?.postMessage(
        {
          type: "execute",
          code: panelSchema.customChartContent,
          data: JSON.stringify(searchQueryData),
        },
        "*"
      );
    };
  });
};


