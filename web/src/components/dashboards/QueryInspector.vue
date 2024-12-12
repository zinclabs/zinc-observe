<template>
  <q-card style="min-width: 700px">
    <q-card-section class="q-pt-md">
      <div class="row items-center">
        <div class="text-bold text-h6 q-pb-lg">Query Inspector</div>
        <q-space />
        <q-btn icon="close" class="q-mb-lg" flat round dense v-close-popup = "true" data-test="query-inspector-close-btn"/>
      </div>
      <div class="text-bold q-pb-sm">Panel : {{ dataTitle }}</div>
      <div class="text-bold">Total Query(s) Executed: {{ totalQueries }}</div>
      <div v-for="(query, index) in ((metaData as any)?.queries ?? [])" :key="query?.originalQuery">
        <div class="text-bold q-py-sm">Query: {{ index + 1 }}</div>
            <q-table class="my-sticky-virtscroll-table" virtual-scroll  v-model:pagination="pagination"
              :rows-per-page-options="[0]" :virtual-scroll-sticky-size-start="48" dense :rows="getRows(query)"
              hide-bottom hide-header row-key="index" wrap-cells data-test="query-inspector">
            </q-table>
      </div>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { computed, defineComponent, ref } from "vue";
import { timestampToTimezoneDate } from "@/utils/zincutils";
import { useStore } from "vuex";

export default defineComponent({
  name: "QueryInspector",
  props: {
    metaData: {
      validator: value => {
        // Custom validation logic
        return typeof value == 'object' || typeof value == undefined;
      },
      required: true
    },
    data: {
      type: Object,
      required: true
    }
  },
  setup(props: any) {
    const queryData = props.metaData?.queries || [] ;
    const store = useStore();
    
    const getRows = (query: any) => {
      
      const timestampOfStartTime = query?.startTime;
      const formattedStartTime = timestampToTimezoneDate(
        timestampOfStartTime / 1000,
        store.state.timezone,
        "yyyy-MM-dd HH:mm:ss.SSS"
      );
      const startTimeEntry = `${timestampOfStartTime} (${formattedStartTime} ${store.state.timezone})`;

      const timestampOfEndTime = query?.endTime;
      const formattedEndTime = timestampToTimezoneDate(
        timestampOfEndTime / 1000,
        store.state.timezone,
        "yyyy-MM-dd HH:mm:ss.SSS"
      );
      const endTimeEntry = `${timestampOfEndTime} (${formattedEndTime} ${store.state.timezone})`;

      const rows: any[] = [
        ["Original Query", query?.originalQuery],
        ["Query", query?.query],
        ["Start Time", startTimeEntry],
        ["End Time", endTimeEntry],
        ["Query Type", query?.queryType],
        ["Variable(s)", ],
        ["Fixed Variable(s)",],
        ["Dynamic Variable(s)",],
      ];

      const variableRows: any[] = [];
      const fixedVariableRows: any[] = [];
      const dynamicVariableRows: any[] = [];

      query?.variables?.forEach((variable: any) => {
        if (variable.type === 'variable') {
          variableRows.push(`${variable.name}: ${variable.value}`);
        } else if (variable.type === 'fixed') {
          fixedVariableRows.push(`${variable.name}: ${variable.value}`);
        } else if (variable.type === 'dynamicVariable') {
          dynamicVariableRows.push(`${variable.name} ${variable.operator} ${variable.value}`);
        }
      });

      rows[5][1] = variableRows.length > 0 ? variableRows.join(', ') : '-';
      rows[6][1] = fixedVariableRows.length > 0 ? fixedVariableRows.join(', ') : '-';
      rows[7][1] = dynamicVariableRows.length > 0 ? dynamicVariableRows.join(', ') : '-';


      return rows;
    };
    const totalQueries = computed(() => queryData.length);
    const dataTitle = computed(() => props.data.title);

    return {
      queryData,
      getRows,
      totalQueries,
      dataTitle,
      pagination: ref({
        rowsPerPage: 0,
      }),
    };
  },
});
</script>

<style lang="scss" scoped></style>
