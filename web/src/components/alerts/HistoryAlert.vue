<template >
    <div 
    class="store.state.theme === 'dark' ? 'dark-theme-history-page' : 'light-theme-history-page'"
     style="width: 100vw; padding: 16px; ">
    <div class="flex items-center">
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

 <div>
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


  
    </div>
    <tenstack-table
        :loading = isLoading
        :columns="columnsToBeRendered || []"
        :rows="dataToBeLoaded || []"
        class="col-8"
        :default-columns="false"
        />
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
import { date } from 'quasar';



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

    const generateColumns = (data) => {
  if (data.length === 0) return [];
  
  const desiredOrder = [
    '_timestamp', 'key', 'start_time', 'end_time', 'next_run_at',
    'is_realtime', 'is_silenced', 'retries', 'status'
  ];

  const keys = Object.keys(data[0])
    .filter(key => key !== 'module' && key !== 'org'); // Remove 'module' and 'org'

  // Order the keys according to the desired order, putting others at the end
  const orderedKeys = desiredOrder.concat(keys.filter(key => !desiredOrder.includes(key)));
  
  return orderedKeys.map(key => {
    let columnWidth = 200; // Default width
    
    // Customize widths for specific columns
    if (key === '_timestamp' || key === 'key') {
      columnWidth = 200;
    } else if (key === 'is_realtime' || key === 'is_silenced' || key === 'retries') {
      columnWidth = 100;
    } else if (key === 'status') {
      columnWidth = 200;
    }
    
    return {
      id: key,
      header: key.charAt(0).toUpperCase() + key.slice(1).replace(/_/g, ' '), 
      accessorKey: key,
      label: key.charAt(0).toUpperCase() + key.slice(1).replace(/_/g, ' '), 
      sortable: true,
      align: "center",
      enableResizing: true,
      size: columnWidth, // Add column size
      meta: {
        closable: false,
        showWrap: false,
        wrapContent: false,
      },
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
          hit.next_run_at = convertUnixToQuasarFormat(hit.next_run_at);
          hit.start_time = convertUnixToQuasarFormat(hit.start_time);
          hit.end_time = convertUnixToQuasarFormat(hit.end_time);
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
    };

    // Watch the searchObj for changes
    
  },

});

</script>


<style scoped>
.info-box {
  display: flex;
  flex-wrap: wrap; /* Allows items to wrap to the next line if there's not enough space */
  border: 2px solid #333; /* Outer border */
  padding: 16px;
  margin:30px;
  border-radius: 8px;
}

.info-item {
  border-right: 1px solid #ccc; /* Separator border */
  padding: 8px 16px;
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
/* Add any component-specific styles here */
</style>
