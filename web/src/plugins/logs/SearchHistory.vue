<template>
    <div
      :class="store.state.theme === 'dark' ? 'dark-theme-history-page' : 'light-theme-history-page'"
      >
  
      <div class="flex tw-justify-between tw-items-center" >
        <div class="flex items-center q-py-sm q-pl-md">
          <div
          data-test="search-history-alert-back-btn"
          class="flex justify-center items-center q-mr-md cursor-pointer"
          style="border: 1.5px solid; border-radius: 50%; width: 22px; height: 22px;"
          title="Go Back"
          @click="closeSearchHistory">
          <q-icon name="arrow_back_ios_new" size="14px" />
        </div>
        <div class="text-h6" data-test="add-alert-title">
         Search History
        </div>
        </div>
        <div class="flex items-center q-py-sm q-pr-md">
          <date-time
              data-test-name="search-history-date-time"
              ref="searchDateTimeRef"
              auto-apply
              :default-type="searchObj.data.datetime.type"
              @on:date-change="updateDateTime"
            />
            <div>
              <q-btn
                color="secondary"
                label="Get History"
                @click="fetchSearchHistory"
                class="q-ml-md"
              />
            </div>
  
        </div>
      </div>
  
      <div class="full-width">
    <q-page>
      <q-table
        ref="qTable"
        :rows="dataToBeLoaded"
        :columns="columnsToBeRendered"
        :pagination="pagination"
        hide-bottom
        row-key="trace_id"
        :rows-per-page-options="[]"
        class="custom-table"
        style="width: 100%;"
      >


      <template v-slot:body="props">
  <q-tr :props="props" @click="triggerExpand(props)">
    <q-td 
      :class="`column-${col.name}`"
      v-for="col in props.cols"
      :key="col.name"
      :props="props"
    >
      {{ col.value }}
    </q-td>
    
  </q-tr>
  <q-tr v-show="expandedRow === props.row.trace_id" :props="props" >

    <q-td colspan="100%">

      <div class="text-left tw-px-2 expanded-content">
        <strong >SQL Query:</strong>

        <div class="tw-flex tw-items-start tw-justify-center" >
       
         <div class="scrollable-content  ">
          <pre style="text-wrap: wrap;">{{ props.row?.sql }}</pre>

         </div>
         <div class="tw-pl-2">
          <q-btn
            @click.stop="copyToClipboard(props.row.sql)"
            size="xs"
            icon="content_copy"
            class="copy-btn tw-py-3"
          />
          <q-btn
            @click.stop="goToLogs(props.row)"
            size="xs"
            label="Go To Logs"
            class="copy-btn tw-py-3 tw-mx-2"
          />
         </div>
        
         
        </div>
      </div>
    </q-td>
  </q-tr>
</template>


      </q-table>
      <div v-if="!isLoading && dataToBeLoaded.length === 0">
        <NoData />
      </div>
      <div
            v-if="isLoading"
            class="text-center full-width full-height q-mt-lg tw-flex tw-justify-center"
          >
            <q-spinner-hourglass color="primary" size="lg" />
          </div>
    </q-page>
    
    </div>
  </div>
 

      <!-- Show NoData component if there's no data to display -->

  </template>
  <script lang="ts">
  //@ts-nocheck
  import { ref, watch, onMounted  , nextTick} from 'vue';
  import {
    timestampToTimezoneDate , b64EncodeUnicode,convertDateToTimestamp } from "@/utils/zincutils";
  import { useRouter, useRoute } from 'vue-router';
  import { useStore } from 'vuex';
  import { defineAsyncComponent ,defineComponent} from 'vue';
  import useLogs from '../../composables/useLogs'
  import TenstackTable from '../../plugins/logs/TenstackTable.vue';
  import searchService from "@/services/search";
  import NoData from "@/components/shared/grid/NoData.vue";
  import DateTime from "@/components/DateTime.vue";
  import { useI18n } from 'vue-i18n';
  import { date , QTable , useQuasar } from 'quasar';
  import type { Ref } from "vue";
  export default defineComponent({
    name: "SearchHistoryComponent",
    components: {
      DateTime,
      NoData,
    },
    props: {
      isClicked: {
        type: Boolean,
        default: false,
      },
    },
    emits: ['closeSearchHistory'],
    methods: {
        closeSearchHistory() {
        this.$emit('closeSearchHistory');
      },
    },
    setup(props, { emit }) {
      const router = useRouter();
      const $q = useQuasar();
      const route = useRoute();
      const store = useStore();
      const {t} = useI18n();
      const qTable: Ref<InstanceType<typeof QTable> | null> = ref(null);
      const searchDateTimeRef = ref(null)
      const { searchObj } = useLogs();
      const dataToBeLoaded :any = ref([]);
      const dateTimeToBeSent : any = ref({});
      const columnsToBeRendered = ref([]);
      const  expandedRow = ref( []); // Array to track expanded rows
      const isLoading = ref(false);

      const pagination = {
            page: 1,
              rowsPerPage: 50 
      }
      const generateColumns = (data: any) => {
     if (data.length === 0) return [];

  // Define the desired column order and names
  const desiredColumns = [
    { key: 'trace_id', label: 'Trace ID' },
    { key: 'start_time', label: 'Start Time' },
    { key: 'end_time', label: 'End Time' },
    { key: 'duration', label: 'Duration' },
    { key: 'took', label: 'Took' },
    { key: 'scan_size', label: 'Scan Size' },
    { key: 'scan_records', label: 'Scan Records' },
    { key: 'cached_ratio', label: 'Cached Ratio' },
    { key: 'sql', label: 'SQL Query' },
  ];

  return desiredColumns.map(({ key, label }) => {
    let columnWidth = 200;
    let align = "center";
    if(key == "scan_records" || key == "cached_ratio" || key == "took"){
      columnWidth = 80
    }
    if(key == "scan_size"){
      columnWidth = 80
    }
    if(key == "duration"){
      columnWidth = 100
    }
    if( key == 'start_time' || key == 'end_time'){
      columnWidth = 200
    }
    if(key == 'sql'){
      columnWidth = 350
    }
    if(key == "trace_id"){
      columnWidth = 250
      align = "left"
    }
   // Custom width for each column
    return {
      name: key,        // Field name
      label: label,     // Column label
      field: key,       // Field accessor
      align: 'center',
      sortable: true,
      style: `max-width: ${columnWidth}px; width: ${columnWidth}px;`,
    };
  });
};
    

      const fetchSearchHistory = async ( ) => {
        columnsToBeRendered.value = [];
        dataToBeLoaded.value = [];
        expandedRow.value = [];
     try {
      const {org_identifier} = router.currentRoute.value.query;
        isLoading.value = true;
        const {startTime, endTime} = dateTimeToBeSent.value;
 
        const response = await searchService.get_history( org_identifier,startTime,endTime,
           );
           columnsToBeRendered.value = generateColumns(response.data.hits);
           response.data.hits.forEach((hit:any)=>{
            hit.duration = calculateDuration(hit.start_time, hit.end_time);
            hit.toBeStoredStartTime = hit.start_time;
            hit.toBeStoredEndTime = hit.end_time;
            hit.start_time = timestampToTimezoneDate(hit.start_time / 1000, store.state.timezone, "yyyy-MM-dd HH:mm:ss.SSS");
            hit.end_time = timestampToTimezoneDate(hit.end_time / 1000, store.state.timezone, "yyyy-MM-dd HH:mm:ss.SSS");
            hit.took  = formatTime(hit.took);
            })
           dataToBeLoaded.value = response.data.hits;
          isLoading.value = false;
     } catch (error) {
      console.log(error, "error")
      isLoading.value = false;
     } finally {
      isLoading.value = false;
     }
        

      };
      const  copyToClipboard = (text) => {
      navigator.clipboard.writeText(text).then(() => {
        $q.notify({
            type: "positive",
            message: "Content Copied Successfully!",
            timeout: 5000,
          });
      }).catch(() => {
          $q.notify({
            type: "negative",
            message: "Error while copy content.",
            timeout: 5000,
          });
      });
    }
      
      const updateDateTime = async (value: any) => {
        const {startTime, endTime} = value;
        dateTimeToBeSent.value = {
          startTime,
          endTime,
        };
      };
      const  formatTime = (took)  => {
      return `${took.toFixed(2)} sec`;
      }
      const calculateDuration = (startTime, endTime) => {
    
    // Calculate the duration in microseconds
    const durationMicroseconds = endTime - startTime;

    // Convert microseconds to seconds
    const durationSeconds = durationMicroseconds / 1e6;

    // Define time unit conversions
    const secondsInMinute = 60;
    const secondsInHour = 3600;
    const secondsInDay = 86400;
    const secondsInMonth = 2592000; // Approximation (30 days)
    const secondsInYear = 31536000; // Approximation (365 days)

    // Convert to the appropriate time unit
    let result = '';

    if (durationSeconds < secondsInMinute) {
      result = `${durationSeconds.toFixed(2)} seconds`;
    } else if (durationSeconds < secondsInHour) {
      const minutes = Math.floor(durationSeconds / secondsInMinute);
      const seconds = durationSeconds % secondsInMinute;
      result = `${minutes} minutes`;
      if (seconds > 0) {
        result += ` and ${seconds.toFixed(2)} seconds`;
      }
    } else if (durationSeconds < secondsInDay) {
      const hours = Math.floor(durationSeconds / secondsInHour);
      const minutes = Math.floor((durationSeconds % secondsInHour) / secondsInMinute);
      result = `${hours} hours`;
      if (minutes > 0) {
        result += ` and ${minutes} minutes`;
      }
    } else if (durationSeconds < secondsInMonth) {
      const days = Math.floor(durationSeconds / secondsInDay);
      const hours = Math.floor((durationSeconds % secondsInDay) / secondsInHour);
      result = `${days} days`;
      if (hours > 0) {
        result += ` and ${hours} hours`;
      }
    } else if (durationSeconds < secondsInYear) {
      const months = Math.floor(durationSeconds / secondsInMonth);
      const days = Math.floor((durationSeconds % secondsInMonth) / secondsInDay);
      result = `${months} months`;
      if (days > 0) {
        result += ` and ${days} days`;
      }
    } else {
      const years = Math.floor(durationSeconds / secondsInYear);
      const months = Math.floor((durationSeconds % secondsInYear) / secondsInMonth);
      result = `${years} years`;
      if (months > 0) {
        result += ` and ${months} months`;
      }
    }

    return result;
    };


      const triggerExpand = (props) =>{
        if (expandedRow.value === props.row.trace_id) {
            expandedRow.value = null;
          } else {
            // Otherwise, expand the clicked row and collapse any other row
            expandedRow.value = props.row.trace_id;
  }
      }
      const   goToLogs = ( row) => {
        const duration_suffix = row.duration.split(" ")[1];
        // emit('closeSearchHistory');
        const stream: string =
        row.stream_name
      const from =
       row.toBeStoredStartTime;
      const to =
       row.toBeStoredEndTime ;
      const refresh = 0;

      const query = b64EncodeUnicode(row.sql);

      router.push({
        path: "/logs",
        query: {
          stream_type: "logs",
          stream,
          period: '15m',
          refresh,
          sql_mode: "true",
          query,
          defined_schemas:"user_defined_schema",
          org_identifier: row.org_id,
          quick_mode: "false",
          show_histogram: "true",
          type: "search_history_re_apply"

        },
      });
    };


      watch(() => props.isClicked, (value) => {
        if(value == true && !isLoading.value){
          fetchSearchHistory();
        }
      });
      return {
        searchObj,
        store,
        generateColumns,
        fetchSearchHistory,
        dataToBeLoaded,
        columnsToBeRendered,
        t,
        route,
        isLoading,
        qTable,
        updateDateTime,
        pagination,  
        searchDateTimeRef,
        expandedRow,
        goToLogs,
        triggerExpand,
        copyToClipboard,
        formatTime,
      };
      // Watch the searchObj for changes
      
    },
  });
  </script>
 <style lang="scss" scoped >
.expanded-content {
  min-width: 100vh;
  max-height: 100vh; /* Set a fixed height for the container */
  overflow: hidden; /* Hide overflow by default */
}

.scrollable-content {
  width: 100%; /* Use the full width of the parent */
  overflow-y: auto; /* Enable vertical scrolling for long content */
  padding: 10px; /* Optional: padding for aesthetics */
  border: 1px solid #ddd; /* Optional: border for visibility */
  height: 100%;
  max-height: 200px;
   /* Use the full height of the parent */
  text-wrap: normal;
}
.q-td {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.custom-table .q-tr > .q-td:first-child {
  text-align: left;
}


 </style>