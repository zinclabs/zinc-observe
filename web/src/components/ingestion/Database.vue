<!-- Copyright 2023 OpenObserve Inc.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
-->

<template>
  <div class="q-ma-md">
    <div class="tw-text-[16px]">
      <div class="tw-font-bold tw-pt-6 tw-pb-2">
        Explore a comprehensive list of data sources and databases on this page, featuring names and direct links to detailed documentation. From SQL Server and Postgres to MongoDB and Snowflake, access essential resources for each database. Dive deeper into the specifics by visiting the provided links for further insights.
      </div>
      <div
        v-for="database in databasesLinks"
        :key="database.name"
        class="tw-py-2"
      >
        {{ database.name }} -
        <a
          :href="database.link"
          class="hover:tw-underline text-primary"
          target="_blank"
          rel="noopener noreferrer"
          >{{ database.link }}</a
        >
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import CopyContent from "@/components/CopyContent.vue";

export default defineComponent({
  name: "DatabasesPage",
  props: {
    currOrgIdentifier: {
      type: String,
    },
    currUserEmail: {
      type: String,
    },
  },
  components: { CopyContent },
  setup(props) {
    const databasesLinks = [
      {
        name: "SQL Server",
        link: "https://short.openobserve.ai/database/sql-server",
      },
      {
        name: "Postgres",
        link: "https://short.openobserve.ai/database/postgres",
      },
      {
        name: "MongoDB",
        link: "https://short.openobserve.ai/database/mongodb",
      },
      {
        name: "Redis",
        link: "https://short.openobserve.ai/database/redis",
      },
      {
        name: "MySQL",
        link: "https://short.openobserve.ai/database/mysql",
      },
      {
        name: "Snowflake",
        link: "https://short.openobserve.ai/database/snowflake",
      },
      {
        name: "Zookeeper",
        link: "https://short.openobserve.ai/database/zookeeper",
      },
      {
        name: "Cassandra",
        link: "https://short.openobserve.ai/database/cassandra",
      },
      {
        name: "Aerospike",
        link: "https://short.openobserve.ai/database/aerospike",
      },
      {
        name: "DynamoDB",
        link: "https://short.openobserve.ai/database/dynamodb",
      },
    ];

    return {
      databasesLinks,
    };
  },
});
</script>
