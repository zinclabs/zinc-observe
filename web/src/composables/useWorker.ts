// Copyright 2023 OpenObserve Inc.

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

export function useWorker() {
  /**
   * Example usage:
    const newFn = (items: number) => {
      let j = 0;
      for (let i = 0; i < items; i++) {
        j += i;
      }

      console.log("newFn", j);
      return j;
    };

    runWorker(999999, newFn).then((res) => {
      console.log("1st Worker done", res);
    });
   *
   * @param data - any data to be passed to the worker
   * @param workerFunction
   * @returns
   */

  function runWorker<T, R>(
    data: T,
    workerFunction: (data: T) => R
  ): Promise<R> {
    return new Promise((resolve, reject) => {
      // Convert the function to a string
      const functionString = workerFunction.toString();

      // Create a blob with the worker's code
      const blob = new Blob(
        [
          `
          self.onmessage = async function(e) {
            const fn = ${functionString};
            const result = fn(e.data);
            postMessage(result);
          };
        `,
        ],
        { type: "application/javascript" }
      );

      // Create a worker from the blob
      const worker = new Worker(URL.createObjectURL(blob), { type: "module" });

      // Send the data to the worker
      worker.postMessage(data);

      // Handle the message received from the worker
      worker.onmessage = function (e) {
        resolve(e.data);
        worker.terminate(); // Clean up the worker
      };

      // Handle any errors
      worker.onerror = function (err) {
        reject(err);
        worker.terminate(); // Clean up the worker
      };
    });
  }

  return { runWorker };
}
