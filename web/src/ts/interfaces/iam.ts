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

export interface Group {
  group_name: string;
  users: User[];
  roles: Role[];
}

export type PermissionType =
  | "AllowAll"
  | "AllowDelete"
  | "AllowGet"
  | "AllowList"
  | "AllowPost"
  | "AllowPut";

export interface Permission {
  object: string; // stream:geo or stream:org_id
  permission: PermissionType;
}

export interface Role {
  role_name: string;
  permissions?: Permission[];
}

export interface Resource {
  name: string;
  permission: {
    AllowAll: {
      show: boolean;
      value: boolean | null;
    };
    AllowList: {
      show: boolean;
      value: boolean | null;
    };
    AllowGet: {
      show: boolean;
      value: boolean | null;
    };
    AllowDelete: {
      show: boolean;
      value: boolean | null;
    };
    AllowPost: {
      show: boolean;
      value: boolean | null;
    };
    AllowPut: {
      show: boolean;
      value: boolean | null;
    };
  };
  display_name: string;
  type: "Type";
  resourceName: string;
  parent: string;
  childs: Resource[];
  entities: Entity[];
  expand?: boolean;
  slotName?: string;
  has_entities?: boolean;
  is_loading?: boolean;
  show?: boolean;
  childName?: string;
  top_level: boolean;
}

export interface Entity {
  name: string;
  permission: {
    AllowAll: {
      show: boolean;
      value: boolean | null;
    };
    AllowGet: {
      show: boolean;
      value: boolean | null;
    };
    AllowDelete: {
      show: boolean;
      value: boolean | null;
    };
    AllowPut: {
      show: boolean;
      value: boolean | null;
    };
    AllowList: {
      show: boolean;
      value: boolean | null;
    };
    AllowPost: {
      show: boolean;
      value: boolean | null;
    };
  };
  display_name: string;
  entities?: Entity[];
  has_entities?: boolean;
  type: "Resource" | "Type";
  resourceName: string;
  expand?: boolean;
  show?: boolean;
  is_loading?: boolean;
  childName?: string;
  top_level: boolean;
  parent?: string;
}

interface User {}
