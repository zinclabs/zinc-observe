import http from "./http";

const synthetics = {
  list: (org_identifier: string) => {
    return http().get(`/api/${org_identifier}/synthetics`);
  },
  getTest: (org_identifier: string, testName: string) => {
    return http().get(`/api/${org_identifier}/synthetics/${testName}`);
  },
  create: (org_identifier: string, payload: any) => {
    return http().post(`/api/${org_identifier}/synthetics`, payload);
  },
  updateTest: (org_identifier: string, payload: any) => {
    return http().put(
      `/api/${org_identifier}/synthetics/${payload.name}`,
      payload
    );
  },
  delete: (org_identifier: string, testName: string) => {
    return http().delete(`/api/${org_identifier}/synthetics/${testName}`);
  },
  triggerReport: (org_identifier: string, reportName: string) => {
    return http().put(
      `/api/${org_identifier}/synthetics/${reportName}/trigger`
    );
  },
  toggleTestState: (
    org_identifier: string,
    reportName: string,
    state: boolean
  ) => {
    return http().put(
      `/api/${org_identifier}/synthetics/${reportName}/enable?value=${state}`
    );
  },
};

export default synthetics;
