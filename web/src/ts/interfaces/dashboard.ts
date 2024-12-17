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

interface Label {
  show: boolean;
  position: string;
  rotate: number;
}

interface MarkLine {
  silent: boolean;
  animation: boolean;
  data: any[];
}

export interface TrellisLayout {
  layout: "vertical" | "horizontal" | "custom" | null;
  num_of_columns: number;
}
export interface SeriesObject {
  name: string;
  label: Label;
  markLine: MarkLine;
  connectNulls: boolean;
  large: boolean;
  color: string | null;
  data: Array<number | null>;
  [key: string]: any;
}
