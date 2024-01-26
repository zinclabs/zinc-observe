import http from "./http";

// ----------- Groups -------------
export const getGroups = (org_identifier: string) => {
  const url = `/api/${org_identifier}/groups`;
  return http().get(url);
};

export const getGroup = (group_name: string, org_identifier: string) => {
  const url = `/api/${org_identifier}/groups/${group_name}`;
  return http().get(url);
};

export const createGroup = (group_name: string, org_identifier: string) => {
  const url = `/api/${org_identifier}/groups`;
  return http({}).post(url, {
    name: group_name,
    users: [],
    roles: [],
  });
};

export const updateGroup = (group: {
  group_name: string;
  org_identifier: string;
  payload: {
    add_roles: string[];
    remove_roles: string[];
    add_users: string[];
    remove_users: string[];
  };
}) => {
  const url = `/api/${group.org_identifier}/groups/${group.group_name}`;
  return http().put(url, group.payload);
};

// ----------- Roles -------------

export const getRoles = (org_identifier: string) => {
  const url = `/api/${org_identifier}/roles`;
  return http().get(url);
};

export const getRole = (role_id: string, org_identifier: string) => {
  const url = `/api/${org_identifier}/roles/${role_id}`;
  return http().get(url);
};

export const createRole = (role_id: string, org_identifier: string) => {
  const url = `/api/${org_identifier}/roles`;
  return http({}).post(url, {
    name: role_id,
  });
};

export const updateRole = (role: {
  role_id: string;
  org_identifier: string;
  payload: any;
}) => {
  const url = `/ api / ${role.org_identifier} / roles / ${role.role_id} / permissions`;
  return http().put(url, role.payload);
};

// ----------- Permissions -------------

export const getPermissions = (role_id: string, org_identifier: string) => {
  const url = `/ api / ${org_identifier} / roles / `;
  return http().get(url);
};
