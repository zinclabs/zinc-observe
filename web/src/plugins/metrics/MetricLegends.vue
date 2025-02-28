<template>
  <q-btn
    data-cy="metric-legends-button"
    size="sm"
    dense
    flat
    class="metric-legends-button"
  >
    <q-icon name="category" class="q-mr-sm" />
    <span>{{ t("search.legendLabel") }}</span>

    <q-menu :class="store.state.theme == 'dark' ? 'theme-dark' : 'theme-light'">
      <q-card flat>
        <q-card-section class="metric-legends-title">
          <div class="label">{{ t("search.legendLabel") }}</div>
        </q-card-section>
        <q-separator />
        <q-card-section class="q-pt-none legends">
          <div class="legend-grid">
            <div
              class="legend-item"
              v-for="(icon, metric) in metricsIconMapping"
              :key="metric"
            >
              <q-icon :name="icon" class="q-mr-sm" />
              <span>{{ metric }}</span>
            </div>
          </div>
        </q-card-section>
      </q-card>
    </q-menu>
  </q-btn>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";

export default defineComponent({
  name: "MetricLegends",
  setup() {
    const { t } = useI18n();
    const store = useStore();

    const metricsIconMapping: Record<string, string> = {
      Summary: "description",
      Gauge: "speed",
      Histogram: "bar_chart",
      Counter: "pin",
    };

    return {
      t,
      store,
      metricsIconMapping,
    };
  },
});
</script>

<style lang="scss" scoped>
.legend-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr); /* Two columns */
  gap: 10px; /* Space between items */
}
.legend-item {
  display: flex;
  align-items: center;
  font-size: 14px;
  padding: 6px 0;
}
.q-btn:before {
  border: 0px solid #d5d5d5;
}
.metric-legends-button {
  cursor: pointer;
  text-transform: capitalize;
  padding: 5px 5px;
  height: 30px;
  font-weight: bold;
  border: 1px solid rgba(89, 96, 178, 0.3);
}
.theme-dark .legend-item {
  color: white;
}
.theme-light .legend-item {
  color: black;
}
</style>
