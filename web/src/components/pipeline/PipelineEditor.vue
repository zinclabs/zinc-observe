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
  <div class="flex justify-between items-center q-pb-sm">
    <div class="flex items-center">
      <div
        data-test="add-pipeline-back-btn"
        class="flex justify-center items-center q-mr-md cursor-pointer"
        style="
          border: 1.5px solid;
          border-radius: 50%;
          width: 22px;
          height: 22px;
        "
        title="Go Back"
        @click="openCancelDialog"
      >
        <q-icon name="arrow_back_ios_new" size="14px" />
      </div>
      <div class="text-h6">{{ pipeline.name }}</div>
    </div>

    <div class="flex justify-end">
      <q-btn
        data-test="add-pipeline-save-btn"
        label="Cancel"
        class="text-bold border q-ml-md"
        padding="sm xl"
        no-caps
        @click="openCancelDialog"
      />

      <q-btn
        data-test="add-pipeline-save-btn"
        label="Save"
        class="text-bold no-border q-ml-md"
        color="secondary"
        padding="sm xl"
        no-caps
        @click="savePipeline"
      />
    </div>
  </div>

  <q-separator class="q-mb-sm" />

  <div class="flex q-mt-md">
    <div class="nodes-drag-container q-pr-md">
      <div
        data-test="pipeline-editor-nodes-list-title"
        class="text-bold q-mb-sm q-mx-sm"
      >
        {{ t("pipeline.nodes") }}
      </div>

      <q-separator class="q-mb-md" />

      <template v-for="nodeType in nodeTypes" :key="nodeType.value">
        <div
          :data-test="`pipeline-editor-${nodeType.value}-node`"
          :id="nodeType.value + '_node'"
          :draggable="true"
          @dragstart="(ev) => onNodeDragStart(ev, nodeType.value)"
          class="flex items-center node-type-row q-mt-sm q-mx-sm"
        >
          <img width="30" :src="symbolMapping[nodeType.value]" />
          <div
            :data-test="`pipeline-editor-${nodeType.value}-node-title`"
            class="q-px-sm"
          >
            {{ nodeType.label }}
          </div>
        </div>
      </template>
    </div>
    <div
      id="pipelineChartContainer"
      ref="chartContainerRef"
      class="relative-position pipeline-chart-container"
      :class="store.state.theme === 'dark' ? '' : 'bg-grey-2'"
    >
      <div
        class="q-pl-sm q-pt-xs"
        :class="store.state.theme === 'dark' ? 'text-white' : 'text-black'"
        data-test="pipeline-editor-execution-note"
      >
        Note: During execution of pipeline, routes will get executed before
        functions
      </div>
      <ChartRenderer
        data-test="pipeline-editor-pipeline-chart"
        :data="plotChart"
        render-type="svg"
        style="height: calc(100vh - 165px)"
        @click="onChartClick"
        @mouseout="onMouseOut"
        @drop="onNodeDrop"
        @dragover="onNodeDragOver"
      />

      <!-- <div ref="nodeActions" id="node-actions" class="node-actions absolute">
      <q-icon name="delete" class="q-mr-xs cursor-pointer" />
      <q-icon name="add" class="cursor-pointer" size="16px" />
    </div> -->
    </div>
  </div>

  <q-dialog v-model="dialog.show" position="right" full-height maximized>
    <div class="stream-routing-dialog-container full-height">
      <StreamRouting
        v-if="dialog.name === 'streamRouting'"
        :stream-name="pipeline.stream_name"
        :stream-type="pipeline.stream_type"
        :editing-route="streamRoutes[editingStreamRouteName]"
        :stream-routes="streamRoutes"
        @update:node="addStreamNode"
        @cancel:hideform="resetDialog"
        @delete:node="deleteNode"
      />

      <AssociateFunction
        v-if="dialog.name === 'associateFunction'"
        :loading="isFetchingFunctions"
        :function-data="functions[editingFunctionName]"
        :functions="functionOptions"
        :associated-functions="associatedFunctions"
        @update:node="addFunctionNode"
        @delete:node="deleteNode"
        @cancel:hideform="resetDialog"
        @add:function="updateNewFunction"
      />
    </div>
  </q-dialog>
  <confirm-dialog
    :title="confirmDialogMeta.title"
    :message="confirmDialogMeta.message"
    @update:ok="confirmDialogMeta.onConfirm()"
    @update:cancel="resetConfirmDialog"
    v-model="confirmDialogMeta.show"
  />
</template>

<script setup lang="ts">
import {
  computed,
  defineAsyncComponent,
  onBeforeMount,
  ref,
  type Ref,
} from "vue";
import { getImageURL } from "@/utils/zincutils";
import StreamRouting from "./StreamRouting.vue";
import AssociateFunction from "./AssociateFunction.vue";
import functionsService from "@/services/jstransform";
import { useStore } from "vuex";
import pipelineService from "@/services/pipelines";
import { useRouter } from "vue-router";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import { useI18n } from "vue-i18n";
import { useQuasar } from "quasar";
import jstransform from "@/services/jstransform";

const functionImage = getImageURL("images/pipeline/function.svg");
const streamImage = getImageURL("images/pipeline/stream.svg");
const streamRouteImage = getImageURL("images/pipeline/route.svg");
const conditionImage = getImageURL("images/pipeline/condition.svg");

const ChartRenderer = defineAsyncComponent(
  () => import("@/components/dashboards/panels/ChartRenderer.vue")
);

interface Routing {
  [key: string]: RouteCondition[];
}

interface RouteCondition {
  column: string;
  operator: string;
  value: string;
}

interface Function {
  name: string;
  description: string;
  stream: string;
  order: number;
}

interface Pipeline {
  name: string;
  description: string;
  stream_name: string;
  stream_type: string;
  routing: Routing;
  functions: Function[];
  derived_streams: any[];
}

interface Node {
  name: string;
  x: number;
  y: number;
  type: string;
  fixed: boolean;
  order?: number;
  stream?: string;
}

interface NodeLink {
  from: string[];
  to: string[];
}

const plotChart: any = ref({
  options: {
    tooltip: {},
    series: [
      {
        type: "graph",
        layout: "none",
        symbolSize: "60",
        roam: false,
        label: {
          show: true,
          position: "bottom",
        },
        draggable: true,
        edgeSymbol: ["circle", "arrow"],
        edgeSymbolSize: [10, 10],
        edgeLabel: {
          fontSize: 20,
        },
        data: [],
        links: [],
        lineStyle: {
          opacity: 0.9,
          width: 2,
          curveness: 0,
        },
      },
    ],
  },
});

const pipeline = ref<Pipeline>({
  name: "",
  stream_type: "",
  description: "",
  stream_name: "",
  routing: {},
  functions: [],
  derived_streams: [],
});

const router = useRouter();

const store = useStore();

const confirmDialogMeta: any = ref({
  show: false,
  title: "",
  message: "",
  data: null,
  onConfirm: () => {},
});

const nodeTypes = [
  { label: "Function", value: "function" },
  { label: "Route", value: "streamRoute" },
];

const nodes: Ref<Node[]> = ref([]);

const nodeLinks = ref<{ [key: string]: NodeLink }>({});

const functions = ref<{ [key: string]: Function }>({});

const functionOptions = ref<string[]>([]);

const streamRoutes = ref<{ [key: string]: any }>({});

const editingStreamRouteName = ref<string>("");

const editingFunctionName = ref<string>("");

const isFetchingFunctions = ref(false);

const chartContainerRef = ref(null);

const nodeRows = ref<(string | null)[]>([]);

const q = useQuasar();

const associatedFunctions: Ref<string[]> = ref([]);

const { t } = useI18n();

const dialog = ref({
  name: "streamRouting",
  show: false,
  title: "Stream Routing",
  message: "",
  okCallback: () => {},
});

const onChartClick = (params: any) => {
  const { type, name } = params.data;

  resetDialog();

  if (type === "streamRoute" || type === "condition") {
    if (type === "condition") editingStreamRouteName.value = name.split(":")[0];
    else editingStreamRouteName.value = name;
    dialog.value.show = true;
    dialog.value.name = "streamRouting";
  }

  if (type === "function") {
    getFunctions();
    editingFunctionName.value = name;
    dialog.value.show = true;
    dialog.value.name = "associateFunction";
  }
};

onBeforeMount(() => {
  getPipeline();

  updateGraph();
});

const getPipeline = () => {
  const route = router.currentRoute.value;

  pipelineService
    .getPipelines(store.state.selectedOrganization.identifier)
    .then(async (response) => {
      await getFunctions();

      const _pipeline = response.data.list.find(
        (pipeline: Pipeline) =>
          pipeline.name === route.query.name &&
          pipeline.stream_name === route.query.stream &&
          pipeline.stream_type === route.query.stream_type
      );

      if (!_pipeline) {
        q.notify({
          message: "Pipeline not found",
          color: "negative",
          position: "bottom",
          timeout: 3000,
        });
        router.replace({
          name: "pipelines",
          query: {
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
        return;
      }

      pipeline.value = {
        ..._pipeline,
        functions: _pipeline?.functions?.list || [],
      };

      setupNodes();
    });
};

const setupNodes = () => {
  nodeRows.value = [null, pipeline.value.stream_name, null];

  // set the base stream node
  const node: Node = {
    name: pipeline.value.stream_name,
    x: 0,
    y: 300,
    type: "stream",
    fixed: true,
  };

  nodes.value.push(node);

  nodeLinks.value[pipeline.value.stream_name] = {
    from: [],
    to: [],
  };

  // set functions nodes
  pipeline.value?.functions
    ?.sort((a, b) => a.order - b.order)
    .forEach((_function) => {
      createFunctionNode(
        _function.name,
        pipeline.value.stream_name,
        _function.order
      );

      functions.value[_function.name] = _function;

      associatedFunctions.value.push(_function.name);
    });

  updateFunctionNodesOrder();

  if (pipeline.value?.routing)
    Object.keys(pipeline.value?.routing).forEach((stream: string) => {
      createStreamRouteNode({
        name: stream,
      });
      streamRoutes.value[stream] = {
        name: stream,
        conditions: pipeline.value?.routing[stream],
        is_real_time: true,
      };
    });

  (pipeline.value.derived_streams || []).forEach((stream: any) => {
    createStreamRouteNode({
      name: stream.name,
    });
    streamRoutes.value[stream.name] = { ...stream };
  });

  updateGraph();
};

const onMouseMove = (params: any) => {};

const onMouseOut = (params: any) => {};

const addFunction = () => {
  getFunctions();
  resetDialog();
  dialog.value.show = true;
  dialog.value.name = "associateFunction";
};

const addStream = () => {
  resetDialog();
  dialog.value.show = true;
  dialog.value.name = "streamRouting";
};

const getFunctionFrom = (stream: string) => {
  let fromNodeName = stream;

  const functions = nodes.value.filter(
    (node) => node.type === "function" && node.stream === stream
  );

  if (functions.length) {
    fromNodeName = functions[functions.length - 1].name;
  }

  return fromNodeName;
};

// TODO OK : create separate functions for each type node i.e. function, streamRoute, stream
const getNodePosition = (type: string, node?: any) => {
  let lastNode = nodes.value.filter((node) => node.type === type).pop();

  // If this if first function node, adding offset to left from the main stream node
  const nodeGap = !lastNode ? 130 : 50;

  if (type === "function" && !lastNode) lastNode = nodes.value[0];

  if (type === "function") {
    if (lastNode?.x?.toString() && lastNode?.y?.toString())
      return {
        x: lastNode.x + nodeGap,
        y: lastNode.y,
      };
    else console.log("Error while getting last node position");
  }

  const isAnyNull = nodeRows.value.every((row) => row !== null);
  if (isAnyNull) {
    nodeRows.value.push(null);
    nodeRows.value.unshift(null);
  }

  if (type === "streamRoute") {
    const centerIndex = Math.ceil(nodeRows.value.length / 2) - 1;

    for (let i = centerIndex - 1; i >= 0; i--) {
      if (nodeRows.value[i] === null) {
        const refRow = nodeRows.value[i + 1];
        const refNode = nodes.value.find((node) => node.name === refRow);

        nodeRows.value[i] = node.name;

        if (refNode?.y?.toString())
          return {
            x: 50,
            y: refNode.y - 50,
          };
        else
          console.log("Error while getting last node position for streamRoute");
      }
    }

    for (let i = centerIndex + 1; i < nodeRows.value.length; i++) {
      if (nodeRows.value[i] === null) {
        const refRow = nodeRows.value[i - 1];
        const refNode = nodes.value.find((node) => node.name === refRow);

        nodeRows.value[i] = node.name;

        if (refNode?.y?.toString())
          return {
            x: 50,
            y: refNode.y + 50,
          };
        else
          console.log("Error while getting last node position for streamRoute");
      }
    }
  }
};

const addFunctionNode = (data: { data: Function }) => {
  const nodeName = data.data.name;

  // TODO OK: Optimize this function node creation
  // Get all function nodes
  // Get the editing node
  // Remove function nodes from the nodes state
  // Create functions nodes again and sort function nodes by order

  const functionNodes = nodes.value.filter((node) => node.type === "function");

  if (editingFunctionName.value) {
    const editedNode = nodes.value.find(
      (node) => node.name === editingFunctionName.value
    );

    if (editedNode) editedNode.order = data.data.order;

    editingFunctionName.value = "";
  } else {
    functionNodes.push(
      createFunctionNode(
        nodeName,
        pipeline.value.stream_name,
        data.data.order
      ) as Node
    );
  }

  nodes.value = nodes.value.filter((node) => node.type !== "function");

  functionNodes
    .sort((a, b) =>
      a.order?.toString() && b.order?.toString()
        ? Number(a.order) - Number(b.order)
        : 0
    )
    .forEach((node) => {
      createFunctionNode(
        node.name,
        node.stream as string,
        node.order as number
      );
    });

  if (!functions.value[nodeName]) {
    functions.value[nodeName] = data.data;
    functionOptions.value.push(nodeName);
  }

  functions.value[nodeName].order = data.data.order;

  associatedFunctions.value.push(nodeName);

  // reorder function nodes with order key and its links too
  updateFunctionNodesOrder();

  updateGraph();
};

const createFunctionNode = (
  nodeName: string,
  streamName: string,
  order: number
) => {
  const position = getNodePosition("function");

  if (position?.x === undefined || position?.y === undefined) {
    console.log("Error in getting node position");
    return;
  }

  const node = {
    name: nodeName,
    x: position.x,
    y: position.y,
    type: "function",
    fixed: true,
    order: order || 1,
    stream: streamName,
  };

  nodes.value.push(node);

  nodeLinks.value[nodeName] = { from: [], to: [] };

  return node;
};

const updateFunctionNodesOrder = () => {
  const sortedFunctionNodes = nodes.value
    .filter((node) => node.type === "function")
    .sort((a, b) =>
      a.order?.toString() && b.order?.toString()
        ? Number(a.order) - Number(b.order)
        : 0
    );

  sortedFunctionNodes.forEach((node, index) => {
    nodeLinks.value[node.name] = { from: [], to: [] };

    if (index === 0) {
      nodeLinks.value[node.name].from = [nodes.value[0].name];
      nodeLinks.value[nodes.value[0].name].to = [node.name];
    } else {
      nodeLinks.value[node.name].from = [sortedFunctionNodes[index - 1].name];
      nodeLinks.value[sortedFunctionNodes[index - 1].name].to = [node.name];
    }
  });
};

const addStreamNode = (data: { data: Node }) => {
  const nodeName = data.data.name;

  streamRoutes.value[nodeName] = data.data;

  if (editingStreamRouteName.value) {
    editingStreamRouteName.value = "";
  } else {
    createStreamRouteNode(data.data);
  }

  updateGraph();
};

const createStreamRouteNode = (streamRouteData: { name: string }) => {
  const position = getNodePosition("streamRoute", streamRouteData);

  const nodeName = streamRouteData.name;

  if (position?.x === undefined || position?.y === undefined) {
    console.log("Error in getting node position");
    return;
  }

  const condition = nodes.value.push({
    name: nodeName + ":condition",
    x: position.x,
    y: position.y,
    type: "condition",
    fixed: true,
  });

  const stream = nodes.value.push({
    name: nodeName,
    x: position.x + 50,
    y: position.y,
    type: "streamRoute",
    fixed: true,
  });

  nodeLinks.value[nodeName + ":condition"] = {
    from: [nodes.value[0].name],
    to: [nodeName],
  };

  nodeLinks.value[nodeName] = { from: [nodeName + ":condition"], to: [] };

  return {
    condition,
    stream,
  };
};

const symbolMapping: { [key: string]: string } = {
  stream: streamImage,
  function: functionImage,
  streamRoute: streamRouteImage,
  condition: conditionImage,
};

const getNodeSymbol = (type: string) => {
  return (
    "image://" +
    window.location.origin +
    (symbolMapping[type].startsWith("/")
      ? symbolMapping[type]
      : "/" + symbolMapping[type])
  );
};

const updateGraph = () => {
  const data = nodes.value.map((node) => ({
    name: node.name,
    x: node.x, // You may want to adjust these manually if needed (like x: 100 for 'k8s_event')
    y: node.y,
    symbol: getNodeSymbol(node.type),
    fixed: node.fixed,
    type: node.type,
    label: {
      show: true,
      position: "bottom",
      fontSize: 12,
      formatter: (params: any) => {
        if (params.data.type === "condition") return "Condition";
        else return params.data.name;
      },
    },
  }));

  // Prepare links from 'nodeLinks'
  const links: any[] = [];
  for (const nodeName in nodeLinks.value) {
    const { to, from } = nodeLinks.value[nodeName];
    from.forEach((source) => {
      links.push({
        source: source,
        target: nodeName,
      });
    });

    to.forEach((target) => {
      links.push({
        source: nodeName,
        target: target,
      });
    });
  }

  plotChart.value.options.series[0].data = data;
  plotChart.value.options.series[0].links = links;
};

const getFunctions = () => {
  if (Object.keys(functions.value).length) return;
  isFetchingFunctions.value = true;
  return functionsService
    .list(
      1,
      100000,
      "name",
      false,
      "",
      store.state.selectedOrganization.identifier
    )
    .then((res) => {
      functions.value = {};
      functionOptions.value = [];
      res.data.list.forEach((func: Function) => {
        functions.value[func.name] = func;
        functionOptions.value.push(func.name);
      });
    })
    .finally(() => {
      isFetchingFunctions.value = false;
    });
};

const deleteNode = (node: { data: Node; type: string }) => {
  nodes.value = nodes.value.filter((n) => n.name !== node.data.name);

  delete nodeLinks.value[node.data.name];

  if (node.type === "function") {
    associatedFunctions.value = associatedFunctions.value.filter(
      (func) => func !== node.data.name
    );
    editingFunctionName.value = "";
  }

  if (node.type === "streamRoute") {
    delete streamRoutes.value[node.data.name];
    editingStreamRouteName.value = "";
  }

  nodeRows.value.forEach((row, index) => {
    if (row === node.data.name) nodeRows.value[index] = null;
  });

  updateFunctionNodesOrder();

  updateGraph();
};

const resetDialog = () => {
  dialog.value.show = false;
  dialog.value.name = "";
  editingFunctionName.value = "";
  editingStreamRouteName.value = "";
};

const getPipelinePayload = () => {
  const payload = {
    name: pipeline.value.name,
    description: pipeline.value.description,
    stream_name: pipeline.value.stream_name,
    stream_type: pipeline.value.stream_type,
    routing: {} as Routing,
    derived_streams: [] as any[],
  };

  Object.values(streamRoutes.value).forEach((route: any) => {
    if (!route.is_real_time) {
      payload.derived_streams.push(route);
      return;
    } else {
      payload.routing[route.name] = route.conditions;
    }
  });

  return payload;
};

const associateFunctions = async () => {
  const associateFunctionPromises = [];

  const previousAssociatedFunctions = pipeline.value.functions.map(
    (func) => func.name
  );

  for (let i = 0; i < associatedFunctions.value.length; i++) {
    const functionIndex = previousAssociatedFunctions.indexOf(
      associatedFunctions.value[i]
    );

    // If function is already associated with the pipeline and order is same, skip.
    // If there is no change in associated function name and order, skip.
    if (functionIndex > -1) {
      if (
        pipeline.value.functions[i].name === associatedFunctions.value[i] &&
        pipeline.value.functions[i].order ===
          functions.value[associatedFunctions.value[i]].order
      )
        continue;
    }

    associateFunctionPromises.push(
      jstransform.apply_stream_function(
        store.state.selectedOrganization.identifier,
        pipeline.value.stream_name,
        pipeline.value.stream_type,
        associatedFunctions.value[i],
        {
          order: Number(functions.value[associatedFunctions.value[i]].order),
        }
      )
    );
  }
  return Promise.all(associateFunctionPromises);
};

const deleteFunctionAssociation = (name: string) => {
  return jstransform.remove_stream_function(
    store.state.selectedOrganization.identifier,
    pipeline.value.stream_name,
    pipeline.value.stream_type,
    name
  );
};

const savePipeline = async () => {
  const payload = getPipelinePayload();

  const dismiss = q.notify({
    message: "Saving pipeline...",
    position: "bottom",
    spinner: true,
  });

  if (pipeline.value.functions.length) {
    const deleteFunctionPromises = [];
    for (let i = 0; i < pipeline.value.functions.length; i++) {
      if (!associatedFunctions.value.includes(pipeline.value.functions[i].name))
        deleteFunctionPromises.push(
          deleteFunctionAssociation(pipeline.value.functions[i].name)
        );
    }

    await Promise.all(deleteFunctionPromises);
  }

  await associateFunctions();

  pipelineService
    .updatePipeline({
      data: payload,
      org_identifier: store.state.selectedOrganization.identifier,
    })
    .then(() => {
      q.notify({
        message: "Pipeline saved successfully",
        color: "positive",
        position: "bottom",
        timeout: 3000,
      });
    })
    .catch((error) => {
      q.notify({
        message: error.response?.data?.message || "Error while saving pipeline",
        color: "negative",
        position: "bottom",
        timeout: 3000,
      });
    })
    .finally(() => {
      dismiss();
    });
};

const openCancelDialog = () => {
  confirmDialogMeta.value.show = true;
  confirmDialogMeta.value.title = t("common.cancelChanges");
  confirmDialogMeta.value.message = "Are you sure you want to cancel changes?";
  confirmDialogMeta.value.onConfirm = () => router.back();
};

const resetConfirmDialog = () => {
  confirmDialogMeta.value.show = false;
  confirmDialogMeta.value.title = "";
  confirmDialogMeta.value.message = "";
  confirmDialogMeta.value.onConfirm = () => {};
  confirmDialogMeta.value.data = null;
};

// Drag n Drop methods

const onNodeDragStart = (event: any, data: any) => {
  event.dataTransfer.setData("text", data);
};

const onNodeDrop = (event: any) => {
  event.preventDefault();
  const nodeType = event.dataTransfer.getData("text");

  if (nodeType === "function") {
    addFunction();
  }

  if (nodeType === "streamRoute") {
    addStream();
  }
};

const onNodeDragOver = (event: any) => {
  event.preventDefault();
};

const updateNewFunction = (_function: Function) => {
  if (!functions.value[_function.name]) {
    functions.value[_function.name] = _function;
    functionOptions.value.push(_function.name);
  }
};
</script>

<style scoped lang="scss">
.node-actions {
  top: 227px;
  left: 119px;
}
.diagram-container {
  border: 1px solid #000;
}

.pipeline-chart-container {
  border-radius: 12px;
  width: calc(100% - 200px);
}

.nodes-drag-container {
  width: 200px;
}

.node-type-row {
  cursor: grab;
  padding: 5px;
  border: 1px solid #e4e4e4;
  border-radius: 8px;
  user-select: none;
}
</style>

<style lang="scss">
.stream-routing-dialog-container {
  min-width: 540px !important;
}
</style>
