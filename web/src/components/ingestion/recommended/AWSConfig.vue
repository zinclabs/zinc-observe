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
    <CopyContent class="q-mt-sm" :content="content" />
    <div class="tw-text-[16px]">
      <div class="tw-font-bold tw-pt-6 tw-pb-2">
        Check further documentation at:
      </div>
      <div
        v-for="awsService in awsServiceLinks"
        :key="awsService.name"
        class="tw-py-2"
      >
        {{ awsService.name }} -
        <a
          :href="awsService.link"
          class="hover:tw-underline text-primary"
          target="_blank"
          rel="noopener noreferrer"
          >{{ awsService.link }}</a
        >
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, type Ref } from "vue";
import config from "../../../aws-exports";
import { useStore } from "vuex";
import { getImageURL } from "../../../utils/zincutils";
import CopyContent from "@/components/CopyContent.vue";

export default defineComponent({
  name: "AWSConfig",
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
    const store = useStore();
    // TODO OK: Create interface for ENDPOINT
    const endpoint: any = ref({
      url: "",
      host: "",
      port: "",
      protocol: "",
      tls: "",
    });

    try {
      const url = new URL(store.state.API_ENDPOINT);
      endpoint.value = {
        url: store.state.API_ENDPOINT,
        host: url.hostname,
        port: url.port || (url.protocol === "https:" ? "443" : "80"),
        protocol: url.protocol.replace(":", ""),
        tls: url.protocol === "https:" ? "On" : "Off",
      };
    } catch (e) {
      console.error("Error while creating end point", e);
    }

    const content = `HTTP Endpoint: ${endpoint.value.url}/aws/${store.state.selectedOrganization.identifier}/default/_kinesis_firehose
Access Key: [BASIC_PASSCODE]`;

    const awsServiceLinks = [
      {
        name: "Application Load Balancer (ALB)",
        link: "https://short.openobserve.ai/aws/alb",
      },
      {
        name: "Cloudwatch Logs",
        link: "https://short.openobserve.ai/aws/cloudwatch-logs",
      },
      {
        name: "Eventbridge/Cloudwatch Events",
        link: "https://short.openobserve.ai/aws/eventbridge",
      },
      {
        name: "Cloudwatch Metrics",
        link: "https://short.openobserve.ai/aws/cloudwatch-metrics",
      },
      {
        name: "VPC Flow Logs",
        link: "https://short.openobserve.ai/aws/vpc-flow-logs",
      },
      {
        name: "EC2 Instance Logs",
        link: "https://short.openobserve.ai/aws/ec2",
      },
      {
        name: "Congnito",
        link: "https://short.openobserve.ai/aws/cognito",
      },
      {
        name: "AWS Network Firewall Logs",
        link: "https://short.openobserve.ai/aws/network-firewall-logs",
      },
      {
        name: "AWS WAF Logs",
        link: "https://short.openobserve.ai/aws/waf",
      },
      {
        name: "RDS Logs",
        link: "https://short.openobserve.ai/aws/rds",
      },
      {
        name: "DynamoDB Logs",
        link: "https://short.openobserve.ai/aws/dynamodb",
      },
      {
        name: "Route53 Logs",
        link: "https://short.openobserve.ai/aws/route53",
      },
      {
        name: "API Gateway Logs",
        link: "https://short.openobserve.ai/aws/api-gateway",
      },
      {
        name: "Cloudfront Logs",
        link: "https://short.openobserve.ai/aws/cloudfront",
      },
      {
        name: "AWS IoT",
        link: "https://docs.aws.amazon.com/firehose/latest/dev/writing-with-iot.html",
      },
      {
        name: "Other AWS Services (Via Kinesis streams)",
        link: "https://docs.aws.amazon.com/streams/latest/dev/using-other-services.html",
      },
    ];

    return {
      store,
      config,
      endpoint,
      content,
      getImageURL,
      awsServiceLinks,
    };
  },
});
</script>
