import { ref, watch } from "vue";
import { annotationService } from "../../services/dashboard_annotations";
import useNotifications from "../useNotifications";
import { getDashboard } from "@/utils/commons";
import { useStore } from "vuex";

export const useAnnotationsData = (
  organization: string,
  dashboardId: string,
  panelId: string,
  folderId: string,
) => {
  // show annotation button
  const isAddAnnotationMode = ref(false);

  // show add annotation dialog
  const isAddAnnotationDialogVisible = ref(false);

  const isEditMode = ref(false);
  const annotationToAddEdit = ref<any>(null);

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
      }
    });

    return allPanels;
  };

  const fetchAllPanels = async () => {
    try {
      const dashboardData = await getDashboard(store, dashboardId, folderId);

      const processedPanels = processTabPanels(dashboardData);

      panelsList.value = processedPanels;
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
