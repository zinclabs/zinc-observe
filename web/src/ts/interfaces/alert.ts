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

export interface Condition {
  column: string;
  ignore_case: null | boolean;
  operator: string;
  value: string;
}

interface TimingFunction {
  unit: "Minutes";
  value: number;
}

// Alert payload which is sent to backend
export interface Alert {
  name: string;
  stream_name: string;
  stream_type: string;
  query_condition: {
    conditions: Array<Condition>;
    sql: string | null;
    promql: string | null;
    type: "sql" | "promql" | "custom";
  };
  destination: Array<string | Destination>;
  trigger_condition: {
    period: number;
    operator: "=" | ">" | "<" | ">=" | "<=" | "!=" | "Contains" | "NotContains";
    threshold: number;
    silence: number;
  };
  is_real_time: boolean;
  enabled: boolean;
  context_attributes: { [key: string]: string };
  description: string;
  uuid?: string;
}

// Alert object which is modified in frontend to display in table and form
export interface AlertListItem {
  "#": number | string;
  name: string;
  stream_name: string;
  stream_type: string;
  enabled: boolean;
  alert_type: string;
  description: string;
  uuid?: string;
}

// Template payload which is sent to backend
export interface Template {
  name: string;
  body: any;
  isDefault?: boolean;
  type: "http" | "email";
  title?: string;
}

// Template object which is modified in frontend to display in table and form
export interface TemplateData extends Template {
  "#"?: number | string;
}

// Destination payload which is sent to backend
export interface Headers {
  [key: string]: string;
}
export interface Destination {
  name: string;
  url: string;
  method: string;
  skip_tls_verify: boolean;
  headers: Headers;
  template: string | Template;
  emails: string;
  type: "http" | "email";
}

export interface DestinationPayload {
  name: string;
  url: string;
  method: string;
  skip_tls_verify: boolean;
  headers: Headers;
  template: string | Template;
  emails: string[];
  type: "http" | "email";
}

// Destination object which is modified in frontend to display in table and form
export interface DestinationData extends Destination {
  "#"?: number | string;
}
