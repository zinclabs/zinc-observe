import { update } from "lodash-es";
import http from "./http";

const pipelines = {
  getPipelines: (org_identifier: string) => {
    const url = `/api/${org_identifier}/pipelines`;
    return http().get(url);
  },

  getPipeline: ({
    name,
    org_identifier,
  }: {
    name: string;
    org_identifier: string;
  }) => {
    const url = `/api/${org_identifier}/pipelines/${name}`;
    return http().get(url);
  },

  deletePipeline: ({
    pipeline_id,
    org_id,
  }: {
    pipeline_id : string;
    org_id: string;
  }) => {
    const url = `/api/${org_id}/pipelines/${pipeline_id}`;
    return http().delete(url);
  },

  createPipeline: ({
    data,
    org_identifier,
  }: {
    data: object;
    org_identifier: string;
  }) => {
    const url = `/api/${org_identifier}/pipelines`;
    return http({}).post(url, data);
  },

  updatePipeline: ({
    org_identifier,
    data,
  }: {
    org_identifier: string;
    data: any;
  }) => {
    const url = `/api/${org_identifier}/pipelines`;
    return http().put(url, data);
  },
  getPipelineStreams: (org_identifier:string) => {
    console.log(org_identifier,"org_identifier")
    const url = `/api/${org_identifier}/pipelines/streams`;
    return http().get(url);
  },
};

export default pipelines;
