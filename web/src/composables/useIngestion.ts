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

import { useStore } from "vuex";
import { ref } from "vue";

const useIngestion = () => {
  const store = useStore();
  
  const endpoint: any = ref({
    url: "",
    host: "",
    port: "",
    protocol: "",
    tls: "",
  });
  const url = new URL(store.state.API_ENDPOINT);
  endpoint.value = {
    url: store.state.API_ENDPOINT,
    host: url.hostname,
    port: url.port || (url.protocol === "https:" ? "443" : "80"),
    protocol: url.protocol.replace(":", ""),
    tls: url.protocol === "https:" ? "On" : "Off",
  };

  const databaseContent = `HTTP Endpoint: ${endpoint.value.url}/api/${store.state.selectedOrganization.identifier}/[STREAM_NAME]/_json
Access Key: Basic [BASIC_PASSCODE]`;

  const databaseDocURLs = {
    sqlServer: "https://short.openobserve.ai/database/sql-server",
    postgres: "https://short.openobserve.ai/database/postgres",
    mongoDB: "https://short.openobserve.ai/database/mongodb",
    redis: "https://short.openobserve.ai/database/redis",
    couchDB: "https://short.openobserve.ai/database/couchdb",
    elasticsearch: "https://short.openobserve.ai/database/elasticsearch",
    mySQL: "https://short.openobserve.ai/database/mysql",
    sapHana: "https://short.openobserve.ai/database/sap-hana",
    snowflake: "https://short.openobserve.ai/database/snowflake",
    zookeeper: "https://short.openobserve.ai/database/zookeeper",
    cassandra: "https://short.openobserve.ai/database/cassandra",
    aerospike: "https://short.openobserve.ai/database/aerospike",
    dynamoDB: "https://short.openobserve.ai/database/dynamodb",
  };


  return {
    endpoint,
    databaseContent,
    databaseDocURLs,
  };
};

export default useIngestion;
