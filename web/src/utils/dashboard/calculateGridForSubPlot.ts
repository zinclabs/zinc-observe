// it is used to create grid array for gauge chart
export function calculateGridPositions(width: any, height: any, numGrids: any) {
  const gridArray: any = [];

  // if no grid then return empty array
  if (numGrids <= 0) {
    return gridArray;
  }
  // Calculate the aspect ratio of the available space
  const aspectRatio = width / height;

  // Calculate the number of rows and columns based on the aspect ratio
  let numRows = Math.ceil(Math.sqrt(numGrids / aspectRatio));
  let numCols = Math.ceil(numGrids / numRows);

  // Adjust the number of rows and columns if necessary to fit all grids
  while ((numRows - 1) * numCols >= numGrids) {
    numRows--;
    numCols = Math.ceil(numGrids / numRows);
  }

  // width and height for single gauge
  const cellWidth = 100 / numCols;
  const cellHeight = 100 / numRows;

  // will create 2D grid array
  for (let row = 0; row < numRows; row++) {
    for (let col = 0; col < numCols; col++) {
      const grid = {
        left: `${col * cellWidth}%`,
        top: `${row * cellHeight}%`,
        width: `${cellWidth}%`,
        height: `${cellHeight}%`,
      };
      gridArray.push(grid);
    }
  }

  // return grid array, width, height, gridNoOfRow, gridNoOfCol
  return {
    gridArray: gridArray,
    gridWidth: (cellWidth * width) / 100,
    gridHeight: (cellHeight * height) / 100,
    gridNoOfRow: numRows,
    gridNoOfCol: numCols,
  };
}

// it is used to create grid array for trellis chart
export function getTrellisGrid(
  width: number,
  height: number,
  numGrids: number,
  leftMargin: number,
  numOfColumns: number = -1,
) {
  if (width <= 0 || height <= 0) {
    throw new Error("Width and height must be positive numbers");
  }

  const gridArray: any = [];

  // if no grid then return empty array
  if (numGrids <= 0) {
    return gridArray;
  }

  // Calculate the aspect ratio of the available space
  const aspectRatio = width / height;

  // Calculate the number of rows and columns based on the aspect ratio
  let numRows = Math.ceil(Math.sqrt(numGrids / aspectRatio));
  let numCols = Math.ceil(numGrids / numRows);

  // Adjust the number of rows and columns if necessary to fit all grids
  while ((numRows - 1) * numCols >= numGrids) {
    numRows--;
    numCols = Math.ceil(numGrids / numRows);
  }

  if (numOfColumns > 0) {
    numCols = numOfColumns;
    numRows = Math.ceil(numGrids / numCols);
  }

  const spaceBetweenCharts = 4;
  const verticalSpacingBetweenCharts = 30;

  // How many cols
  const xSpacingBetween = ((spaceBetweenCharts * (numCols - 1)) / width) * 100;
  const ySpacingBetween =
    ((verticalSpacingBetweenCharts * (numRows - 1)) / height) * 100;
  const topPadding = (20 / height) * 100;
  const bottomPadding = (52 / height) * 100;

  const leftPadding = ((leftMargin + 16) / width) * 100;
  const rightPadding = (16 / width) * 100;

  // width and height for single gauge
  const cellWidth =
    (100 - (xSpacingBetween + rightPadding + leftPadding)) / numCols;
  const cellHeight =
    (100 - (ySpacingBetween + topPadding + bottomPadding)) / numRows;

  // will create 2D grid array
  for (let row = 0; row < numRows; row++) {
    for (let col = 0; col < numCols; col++) {
      if (gridArray.length > numGrids) {
        break;
      }
      const grid = {
        left: `${col * cellWidth + col * ((spaceBetweenCharts / width) * 100) + leftPadding}%`,
        top: `${row * cellHeight + row * ((verticalSpacingBetweenCharts / height) * 100) + topPadding}%`,
        width: `${cellWidth}%`,
        height: `${cellHeight}%`,
      };
      gridArray.push(grid);
    }
  }

  // return grid array, width, height, gridNoOfRow, gridNoOfCol
  return {
    gridArray: gridArray,
    gridWidth: (cellWidth * width) / 100,
    gridHeight: (cellHeight * height) / 100,
    gridNoOfRow: numRows,
    gridNoOfCol: numCols,
  };
}
