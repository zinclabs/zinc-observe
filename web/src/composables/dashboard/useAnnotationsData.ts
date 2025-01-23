import { ref, watch } from "vue";
import { annotationService } from "../../services/dashboard_annotations";
import useNotifications from "../useNotifications";
import { getDashboard } from "@/utils/commons";
import { useStore } from "vuex";

export const useAnnotationsData = (
  organization: any,
  dashboardId: any,
  panelId: string,
) => {
  // show annotation button
  const isAddAnnotationMode = ref(false);

  // show add annotation dialog
  const isAddAnnotationDialogVisible = ref(false);

  const isEditMode = ref(false);
  const annotationToAddEdit = ref<any>(null);

  const annotations = ref<any[]>([]);

  const { showInfoNotification } = useNotifications();

  // Function

  // show add annotation button
  const enableAddAnnotationMode = () => {
    isAddAnnotationMode.value = true;
  };

  // hide add annotation button
  const disableAddAnnotationMode = () => {
    isAddAnnotationMode.value = false;
  };

  const toggleAddAnnotationMode = () => {
    isAddAnnotationMode.value = !isAddAnnotationMode.value;
  };

  // show annoation dialog
  const showAddAnnotationDialog = () => {
    isAddAnnotationDialogVisible.value = true;
  };

  // hide annotation dialog
  const hideAddAnnotationDialog = () => {
    isAddAnnotationDialogVisible.value = false;
    isEditMode.value = false;
    annotationToAddEdit.value = null;
  };

  const handleAddAnnotationButtonClick = () => {
    disableAddAnnotationMode();
    isEditMode.value = false;
    annotationToAddEdit.value = null;
    showAddAnnotationDialog();
  };

  // Handle adding or editing annotation
  const handleAddAnnotation = (start: any, end: any) => {
    annotationToAddEdit.value = {
      start_time: start ? Math.trunc(start * 1000) : null,
      end_time: end ? Math.trunc(end * 1000) : null,
      title: "",
      text: "",
      tags: [],
      panels: [panelId],
    };

    showAddAnnotationDialog();
  };

  const editAnnotation = (annotation: any) => {
    console.log("Editing annotation:", annotation);
    
    annotationToAddEdit.value = annotation;
    showAddAnnotationDialog();
  };

  // Dialog close handler
  const closeAddAnnotation = () => {
    isAddAnnotationDialogVisible.value = false;
    isAddAnnotationMode.value = false;
    isEditMode.value = false;
    annotationToAddEdit.value = null;
  };

  // Watch for annotation mode to show notification
  watch(isAddAnnotationMode, () => {
    if (isAddAnnotationMode.value) {
      showInfoNotification(
        "Click on the chart data or select a range to add an annotation",
        {},
      );
    }
  });

  const store = useStore();
  const panelsList = ref<any[]>([]);
  const chartTypes = [
    "area",
    "area-stacked",
    "bar",
    "h-bar",
    "line",
    "scatter",
    "stacked",
    "h-stacked",
  ];
  const processTabPanels = (dashboardData: any): any[] => {
    if (!dashboardData?.tabs || !Array.isArray(dashboardData.tabs)) {
      console.warn("No tabs found in dashboard data");
      return [];
    }

    const allPanels: any[] = [];

    dashboardData.tabs.forEach((tab: any) => {
      const tabName = tab.name?.trim() || "Unnamed Tab";

      if (tab.panels && Array.isArray(tab.panels)) {
        const tabPanels = tab.panels
          .filter((panel: any) => chartTypes.includes(panel.type))
          .map((panel: any) => ({
            ...panel,
            tabName: tabName,
            originalTabData: {
              tabId: tab.tabId,
              name: tab.name,
            },
          }));

        allPanels.push(...tabPanels);
      } else {
        console.log(`Tab "${tabName}" has no panels`);
      }
    });

    return allPanels;
  };

  const fetchAllPanels = async () => {
    try {
      const dashboardData = await getDashboard(store, dashboardId);

      const processedPanels = processTabPanels(dashboardData);

      panelsList.value = processedPanels;

      console.log(
        "Processed Panels:",
        processedPanels.map((p) => ({
          id: p.id,
          title: p.title,
          tabName: p.tabName,
          type: p.type,
        })),
      );
    } catch (error) {
      console.error("Error fetching panels:", error);
      panelsList.value = [];
    }
  };

  return {
    isAddAnnotationMode,
    isAddAnnotationDialogVisible,
    isEditMode,
    annotationToAddEdit,
    annotations,
    editAnnotation,
    enableAddAnnotationMode,
    disableAddAnnotationMode,
    toggleAddAnnotationMode,
    showAddAnnotationDialog,
    hideAddAnnotationDialog,
    handleAddAnnotation,
    handleAddAnnotationButtonClick,
    closeAddAnnotation,
    fetchAllPanels,
    panelsList,
  };
};
