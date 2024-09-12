<template>
  <div
    :class="store.state.theme === 'dark' ? 'dark-theme-history-page' : 'light-theme-history-page'"
    style="width: 100vw;">
    
    <div class="flex tw-justify-between tw-items-center" >
      <div class="flex items-center q-py-sm q-pl-md">
        <div
        data-test="add-alert-back-btn"
        class="flex justify-center items-center q-mr-md cursor-pointer"
        style="border: 1.5px solid; border-radius: 50%; width: 22px; height: 22px;"
        title="Go Back"
        @click="closeDialog">
        <q-icon name="arrow_back_ios_new" size="14px" />
      </div>
      <div class="text-h6" data-test="add-alert-title">
        {{ t("alerts.historyTitle") }}
      </div>
      </div>
      <div class="flex items-center q-py-sm q-pr-md">
        <date-time
            ref="dateTimeRef"
            auto-apply
            :default-type="searchObj.data.datetime.type"
            :default-absolute-time="{
              startTime: searchObj.data.datetime.startTime,
              endTime: searchObj.data.datetime.endTime,
            }"
            :default-relative-time="searchObj.data.datetime.relativeTimePeriod"
            data-test="alerts-history-date-time-dropdown"
            @on:date-change="updateDateTime"
            @on:timezone-change="updateTimezone"
          />
          <div>
            <q-btn
              color="secondary"
              label="Get History"
              @click="fetchAlertHistory"
              class="q-ml-md"
            />
          </div>
          
      </div>
    </div>

    <div class="q-mx-md" v-if="dataToBeLoaded.length > 0">
      <div class="info-box">
        <div class="info-item">
          <div class="info-heading">Alert Name</div>
          <div class="info-value">{{ route.query.name }}</div>
        </div>
        <div class="info-item">
          <div class="info-heading">Org</div>
          <div class="info-value">{{ route.query.org_identifier }}</div>
        </div>
        <div class="info-item">
          <div class="info-heading">Next Run</div>
          <div class="info-value">{{ dataToBeLoaded[0]?.next_run_at || 'No History' }}</div>
        </div>
        <div class="info-item">
          <div class="info-heading">Is Realtime</div>
          <div class="info-value">{{ dataToBeLoaded[0]?.is_realtime || 'False' }}</div>
        </div>
        <div class="info-item">
          <div class="info-heading">Is Silence</div>
          <div class="info-value">{{ dataToBeLoaded[0]?.is_silenced || 'False' }}</div>
        </div>
      </div>
    </div>

    <div class="full-width"  v-if="!isLoading && dataToBeLoaded.length > 0">
      <q-table
        ref="qTable"
        hide-bottom
        :rows="dataToBeLoaded"
        :columns="columnsToBeRendered"
        :pagination="pagination"

        row-key="id"
        :rows-per-page-options=[]
        class=" custom-table"
        style="width: 100% ;">
        
        <template #body-cell-status="props">
          <q-td :props="props">
            {{ props.row.status }}
            <q-icon
              v-if="props.row.status === 'failed'"
              name="warning"
              color="red"
              class="q-ml-sm">
              <q-tooltip>
                {{ props.row.error || 'Alert Error' }}
              </q-tooltip>
            </q-icon>
          </q-td>
        </template>
      </q-table>
    </div>

    <!-- Show NoData component if there's no data to display -->
    <div v-if="!isLoading && dataToBeLoaded.length === 0">
      <NoData />
    </div>
    <div
          v-if="isLoading"
          class="text-center full-width full-height q-mt-lg tw-flex tw-justify-center"
        >
          <q-spinner-hourglass color="primary" size="lg" />
        </div>
  </div>
</template>

<script lang="ts">
import { ref, watch, onMounted  , nextTick} from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { useStore } from 'vuex';
import { defineAsyncComponent ,defineComponent} from 'vue';
import useLogs from '../../composables/useLogs'
import TenstackTable from '../../plugins/logs/TenstackTable.vue';
import alertsService from "@/services/alerts";
import NoData from "@/components/shared/grid/NoData.vue";
import DateTime from "@/components/DateTime.vue";
import { useI18n } from 'vue-i18n';
import { date , QTable , useQuasar } from 'quasar';


import type { Ref } from "vue";


export default defineComponent({
  name: "AlertHistory",
  components: {
    TenstackTable: defineAsyncComponent(() => import("../../plugins/logs/TenstackTable.vue")),
    DateTime,
    NoData,
  },
  props: {
    alertName: String,
    alertTitle: String,
    additionalInfo: String,
  },



  emits: ['closeDialog'],
  methods: {

    closeDialog() {
      this.$emit('closeDialog');
    },
  },
  setup(props, { emit }) {
    const router = useRouter();
    const $q = useQuasar();

    const route = useRoute();
    const store = useStore();
    const {t} = useI18n();
    const qTable: Ref<InstanceType<typeof QTable> | null> = ref(null);
    const dateTimeRef = ref(null)
    // Fetch the logs
    const { searchObj } = useLogs();
    const dataToBeLoaded :any = ref([]);
    const dateTimeToBeSent : any = ref({});
    const columnsToBeRendered :any = ref([]);
    const isLoading = ref(true);
const pagination = {
      page: 1,
        rowsPerPage: 50 
}

    const desiredOrder = [
      'triggered_at','start_time', 'end_time',
 'retries', 'status'
  ];
    const generateColumns = (data :any) => {
  if (data.length === 0) return [];
  

  const keys = Object.keys(data[0])
    .filter(key => key !== 'module' && key !== '_timestamp' && key !== 'key' && key !== 'org' && key !== 'next_run_at' && key !== 'is_realtime' && key !== 'is_silenced'); // Remove 'module' and 'org'

  const orderedKeys = desiredOrder.concat(keys.filter(key => !desiredOrder.includes(key)));
  
  return orderedKeys.map(key => {
    let columnWidth = 250; // Default width
    
    // Customize widths for specific columns
if(key === 'status'){
  columnWidth = 400;
}
if(key === 'retries'){
  columnWidth = 120;
}
    return {
      name: key,  // field name
      label: key.charAt(0).toUpperCase() + key.slice(1).replace(/_/g, ' '),  // Column label
      field: key,  // Field accessor
      align: "center",
      sortable: true,
      style: `width: ${columnWidth}px`,  // Custom width for each column
    };
  });
};




const convertUnixToQuasarFormat = (unixMicroseconds : any) => {
      if (!unixMicroseconds) return "";
      const unixSeconds = unixMicroseconds / 1e6;
      const dateToFormat = new Date(unixSeconds * 1000);
      const formattedDate = dateToFormat.toISOString();
      return date.formatDate(formattedDate, "YYYY-MM-DDTHH:mm:ssZ");
    }
    const fetchAlertHistory = async ( ) => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading history...",
      });
      const { org_identifier, name, stream_name } = route.query;
      isLoading.value = true;
      if (!org_identifier || !name || !stream_name) {
          return;
        }
      try {
        dismiss();
        const {startTime, endTime} = dateTimeToBeSent.value;
        const response = await alertsService.history(org_identifier , stream_name, name, startTime, endTime);
        if(response.data.hits.length === 0){
          dataToBeLoaded.value = [];
          columnsToBeRendered.value = [];
          isLoading.value = false;
         return;
        }
        response.data.hits.forEach((hit) => {
          hit._timestamp = convertUnixToQuasarFormat(hit._timestamp);
          hit.triggered_at = hit._timestamp;

          hit.next_run_at = convertUnixToQuasarFormat(hit.next_run_at);
          hit.start_time = convertUnixToQuasarFormat(hit.start_time);
          hit.end_time = convertUnixToQuasarFormat(hit.end_time);
        });


        dataToBeLoaded.value =  response.data.hits;

        
        columnsToBeRendered.value = generateColumns(dataToBeLoaded.value);

        isLoading.value = false;

      } catch (error) {
        isLoading.value = false;
        dataToBeLoaded.value = [];
        columnsToBeRendered.value = [];
        console.error(error);
          $q.notify({
            type: "negative",
            message: "Error while pulling history.",
            timeout: 2000,
          });
      }
      finally {
        isLoading.value = false;
      }
    };
    
    const updateDateTime = async (value: object) => {
      if (
        value.valueType == "absolute" &&
        searchObj.data.stream.selectedStream.length > 0 &&
        searchObj.data.datetime.queryRangeRestrictionInHour > 0 &&
        value.hasOwnProperty("selectedDate") &&
        value.hasOwnProperty("selectedTime") &&
        value.selectedDate.hasOwnProperty("from") &&
        value.selectedTime.hasOwnProperty("startTime")
      ) {
        // Convert hours to microseconds
        let newStartTime =
          parseInt(value.endTime) -
          searchObj.data.datetime.queryRangeRestrictionInHour *
            60 *
            60 *
            1000000;

        if (parseInt(newStartTime) > parseInt(value.startTime)) {
          value.startTime = newStartTime;

          value.selectedDate.from = timestampToTimezoneDate(
            value.startTime / 1000,
            store.state.timezone,
            "yyyy/MM/DD",
          );
          value.selectedTime.startTime = timestampToTimezoneDate(
            value.startTime / 1000,
            store.state.timezone,
            "HH:mm",
          );

          dateTimeRef.value.setAbsoluteTime(value.startTime, value.endTime);
          dateTimeRef.value.setDateType("absolute");
        }
      }
      const {startTime, endTime} = value;
      dateTimeToBeSent.value = {
        startTime,
        endTime,
      };

      await nextTick ();
      await nextTick();
      await nextTick();
      await nextTick();
    };
    const updateTimezone = () => {
      if (store.state.zoConfig.query_on_stream_selection == false) {
        emit("onChangeTimezone");
      }
    };
    onMounted(() => {
      fetchAlertHistory(); 
    }),
    watch(
      () => route.query,
      (newParams, oldParams) => {
        if (newParams !== oldParams) {
          fetchAlertHistory(); 
        }
      }
    )



    return {
      searchObj,
      store,
      generateColumns,
      fetchAlertHistory,
      dataToBeLoaded,
      columnsToBeRendered,
      t,
      route,
      isLoading,
      desiredOrder,
      qTable,
      updateDateTime,
      updateTimezone,  
      pagination,  
      dateTimeRef,
    };

    // Watch the searchObj for changes
    
  },

});

</script>


<style lang="scss" scoped >
.info-box {
  display: flex;
  flex-wrap: wrap; /* Allows items to wrap to the next line if there's not enough space */
  border: 2px solid #333; /* Outer border */
  margin-top: 10px;
  border-radius: 8px;
}

.info-item {
  border-right: 1px solid #ccc; 
  padding: 16px;
  /* Separator border */
  flex: 1 1 20%; /* Makes each item take up a quarter of the width, adjust as needed */
  box-sizing: border-box; /* Ensures padding and border are included in the item's width */
}

.info-item:last-child {
  border-right: none; /* Remove the border for the last item */
}

.info-heading {
  font-weight: bold;
  margin-bottom: 4px;
}

.info-value {
  font-size: 14px;
}
.dark-theme-history-page{
  background-color: #181A1B;
}
.light-theme-history-page{
  background-color: #ffffff;
}


.custom-table {
  border-radius: 0px;
  font-size: 18px;  
  margin-top: 10px;


}



/* Add any component-specific styles here */
</style>
