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

<div v-if="!dataToBeLoaded.length == 0" style="{
          height: !searchObj.meta.showHistogram
            ? 'calc(100% - 40px)'
            : 'calc(100% - 140px)',
        }"    >

  <tenstack-table
        :columns="columnsToBeRendered || []"
        :rows="dataToBeLoaded || []"
        :wrap="true"
        class="col-12"
        :default-columns="false"
        />
</div>
<div v-else>
  no data
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


import { useI18n } from 'vue-i18n';



export default defineComponent({
  name: "AlertHistory",
  components: {
    TenstackTable: defineAsyncComponent(() => import("../../plugins/logs/TenstackTable.vue")),
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

const generateColumns = (data) => {
  if (data.length === 0) return [];
  
  const keys = Object.keys(data[0]);
  
  return keys.map(key => ({
    id: key,
    header: key.charAt(0).toUpperCase() + key.slice(1).replace(/_/g, ' '), 
    accessorKey: key,
    label: key.charAt(0).toUpperCase() + key.slice(1).replace(/_/g, ' '), 
    sortable: true,
    enableResizing: true,
    meta: {
      closable: false,
      showWrap: false,
      wrapContent: false,
    },
            size: 150,
  }));
};

    const fetchAlertHistory = async () => {
      const { org_identifier, name, stream_name } = route.query;
      if (!org_identifier || !name || !stream_name) {
          console.log("Skipping fetch due to missing params");
          return;
        }
      try {
        const response = await alertsService.history(org_identifier, stream_name, name);
        dataToBeLoaded.value =  response.data.hits;
        if(dataToBeLoaded.value.length === 0){
         console.log("No data found");
         return;
        }
        columnsToBeRendered.value = generateColumns(dataToBeLoaded.value);
        console.log(dataToBeLoaded.value, "dataToBeLoaded");
        console.log(columnsToBeRendered.value, "columnsToBeRendered");
      } catch (error) {
        this.$q.notify({
          type: "negative",
          message: error.response?.data?.message || "Error fetching history",
        });
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
