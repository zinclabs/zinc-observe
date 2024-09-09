<template >
    <div 
    class="store.state.theme === 'dark' ? 'dark-theme-history-page' : 'light-theme-history-page'"
     style="width: 100vw;  ">
    <div class="flex items-center q-pl-md">
        <div
          data-test="add-alert-back-btn"
          class="flex justify-center items-center q-mr-md cursor-pointer"
          style="
            border: 1.5px solid;
            border-radius: 50%;
            width: 22px;
            height: 22px;
          "
          title="Go Back"
          @click="closeDialog"
        >
          <q-icon name="arrow_back_ios_new" size="14px" />
        </div>
        <div class="text-h6" data-test="add-alert-title">
          {{ t("alerts.historyTitle") }}
        </div>
      </div>

 <div class="q-mx-md ">
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
      <div class="info-value">{{ dataToBeLoaded[0]?.is_realtime  || false}}</div>
    </div>
    <div class="info-item">
      <div class="info-heading">Is Silence</div>
      <div class="info-value">{{ dataToBeLoaded[0]?.is_silenced || false }}</div>
    </div>
  </div>
 </div>

 <div class="full-width">
    <q-table
      :rows="dataToBeLoaded || []"
      :columns="columnsToBeRendered || []"
      :loading="isLoading"
      row-key="id"
      class="col-8 custom-table"
      :hide-bottom="true"
      style="width: 100%"> 

      <template #body-cell-status="props">
        <q-td :props="props">
          {{ props.row.status }}

          <!-- Exclamatory icon with tooltip for 'failed' status -->
          <q-icon
            v-if="props.row.status === 'failed'"
            name="warning"
            color="red"
            class="q-ml-sm"
          >
            <!-- Tooltip to show error message on hover -->
            <q-tooltip>
              {{ props.row.error || 'Unknown error' }}
            </q-tooltip>
          </q-icon>
        </q-td>
      </template>
    </q-table>
  </div>
  
    </div>
 
</template>

<script>
import { ref, watch, onMounted } from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { useStore } from 'vuex';
import { defineAsyncComponent ,defineComponent} from 'vue';
import useLogs from '../../composables/useLogs'
import TenstackTable from '../../plugins/logs/TenstackTable.vue';
import alertsService from "@/services/alerts";
import NoData from "@/components/shared/grid/NoData.vue";

import DateTime from "@/components/DateTime.vue";
import { useI18n } from 'vue-i18n';
import { date , QTable } from 'quasar';



export default defineComponent({
  name: "AlertHistory",
  components: {
    TenstackTable: defineAsyncComponent(() => import("../../plugins/logs/TenstackTable.vue")),
    DateTime,
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
    const route = useRoute();
    const store = useStore();
    const {t} = useI18n();
    

    // Fetch the logs
    const { searchObj } = useLogs();
    const dataToBeLoaded = ref([]);

    const columnsToBeRendered = ref([]);
    const isLoading = ref(false);
    const desiredOrder = [
    '_timestamp', 'start_time', 'end_time',
 'retries', 'status'
  ];
    const generateColumns = (data) => {
  if (data.length === 0) return [];
  
 

  const keys = Object.keys(data[0])
    .filter(key => key !== 'module' && key !== 'key' && key !== 'org' && key !== 'next_run_at' && key !== 'is_realtime' && key !== 'is_silenced'); // Remove 'module' and 'org'

  const orderedKeys = desiredOrder.concat(keys.filter(key => !desiredOrder.includes(key)));
  
  return orderedKeys.map(key => {
    let columnWidth = 120; // Default width
    
    // Customize widths for specific columns

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




const convertUnixToQuasarFormat = (unixMicroseconds) => {
      if (!unixMicroseconds) return "";
      const unixSeconds = unixMicroseconds / 1e6;
      const dateToFormat = new Date(unixSeconds * 1000);
      const formattedDate = dateToFormat.toISOString();
      return date.formatDate(formattedDate, "YYYY-MM-DDTHH:mm:ssZ");
    }
    const fetchAlertHistory = async () => {
      const { org_identifier, name, stream_name } = route.query;
      isLoading.value = true;
      if (!org_identifier || !name || !stream_name) {
          console.log("Skipping fetch due to missing params");
          return;
        }
      try {
        const response = await alertsService.history(org_identifier, stream_name, name);
        if(response.data.hits.length === 0){
         console.log("No data found");
         return;
        }
        response.data.hits.forEach((hit) => {
          hit._timestamp = convertUnixToQuasarFormat(hit._timestamp);

          hit.next_run_at = convertUnixToQuasarFormat(hit.next_run_at);
          hit.start_time = convertUnixToQuasarFormat(hit.start_time);
          hit.end_time = convertUnixToQuasarFormat(hit.end_time);
          hit.status = "failed";
        });

        dataToBeLoaded.value =  response.data.hits;

        
        columnsToBeRendered.value = generateColumns(dataToBeLoaded.value);
        isLoading.value = false;
        console.log(dataToBeLoaded.value, "dataToBeLoaded");
        console.log(columnsToBeRendered.value, "columnsToBeRendered");
      } catch (error) {
        console.error("Error fetching alert history", error);
      }
      finally {
        isLoading.value = false;
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
    );



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
  font-size: 18px;  
  margin-top: 10px;
  border-bottom: 2px  solid #ccc;
  border-top: 2px solid #ccc;

}



/* Add any component-specific styles here */
</style>
