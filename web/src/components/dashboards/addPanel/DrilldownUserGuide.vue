<template>
  <div ref="userGuideBtnRef">
    <q-btn
      no-caps
      @click="onUserGuideClick"
      padding="xs"
      class="q-ml-md"
      flat
      icon="help_outline"
      data-test="dashboard-drilldown-help-btn"
    >
      <q-tooltip class="bg-grey-8" anchor="bottom middle" self="top middle">
        User Guide
      </q-tooltip>
    </q-btn>
  </div>
  <div
    class="user-guide scroll o2-input"
    v-show="showUserGuide"
    style="
      position: absolute;
      z-index: 1;
      width: 500px;
      max-height: 300px;
      border: 1px solid gray;
      border-radius: 5px;
    "
    @mouseleave="showUserGuide = false"
    :class="
      store.state.theme == 'dark'
        ? 'theme-dark bg-dark'
        : 'theme-light bg-white'
    "
    ref="userGuideDivRef"
  >
    <p>
      In URL or while drilldown to another dashboard, you can use the following
      dynamic variables:
    </p>

    <div class="header">Use current dashboard's variable</div>
    <p>You can reference a variable with the following format:</p>
    <ul>
      <li>
        <span class="bg-highlight">${variable_name}</span>
        <br />
        (For Example, if your variable name is "test", you can use
        <span class="bg-highlight">${test}</span>)
      </li>
    </ul>

    <br />

    <div class="header">Use current selected time period</div>
    <p>
      You can reference current selected Time period with the following format:
    </p>
    <ul>
      <li><span class="bg-highlight">${start_time}</span></li>
      <li><span class="bg-highlight">${end_time}</span></li>
    </ul>
    For Example:
    <span class="bg-highlight">from=${start_time}&to=${end_time}</span>
    <br />
    <span class="header">Note: </span>
    <span
      >Even with a relative time period, you can still use
      <span class="bg-highlight">start_time</span> and
      <span class="bg-highlight">end_time</span>.</span
    >
    <br />
    <br />
    <div class="header">Use Series name and value</div>
    <p>
      You can reference the series name and value with the following variables:
    </p>
    <ul>
      <li><span class="bg-highlight">${series.__name}</span></li>
      <li><span class="bg-highlight">${series.__value}</span></li>
    </ul>

    <br />

    <div class="header">For table chart drilldown</div>
    <p>
      You can reference the row field and index with the following variables:
    </p>
    <ul>
      <li>
        <span class="bg-highlight"
          >${row.field["field_label"]} or ${row.field.field_label}</span
        >
        <br />
        (For Example, if your want to use "test" column's value of clicked row,
        you can use <span class="bg-highlight">${row.field.test} </span> or
        <span class="bg-highlight">${row.field["test"]}</span>)
      </li>
      <li><span class="bg-highlight">${row.index}</span></li>
    </ul>

    <br />

    <div class="header">For Sankey chart drilldown</div>
    <p>
      You can reference the edge source, target, and value, as well as the node
      name and value, with the following variables:
    </p>
    <ul>
      <li class="header">Edge</li>
      <ul>
        <li><span class="bg-highlight">${edge.__source}</span></li>
        <li><span class="bg-highlight">${edge.__target}</span></li>
        <li><span class="bg-highlight">${edge.__value}</span></li>
      </ul>
      <li class="header">Node</li>
      <ul>
        <li><span class="bg-highlight">${node.__name}</span></li>
        <li><span class="bg-highlight">${node.__value}</span></li>
      </ul>
    </ul>
  </div>
</template>

<script lang="ts">
import { ref } from "vue";
import { useStore } from "vuex";

export default {
  name: "DrilldownUserGuide",
  setup() {
    const store = useStore();
    const showUserGuide = ref(false);

    const userGuideBtnRef: any = ref(null);
    const userGuideDivRef: any = ref(null);

    const onUserGuideClick = () => {
      if (userGuideBtnRef.value && userGuideDivRef.value) {
        userGuideDivRef.value.style.top =
          userGuideBtnRef.value.getBoundingClientRect().top + 32 + "px";

        userGuideDivRef.value.style.left =
          userGuideBtnRef.value.getBoundingClientRect().left + 48 + "px";
        showUserGuide.value = !showUserGuide.value;
      }
    };

    return {
      store,
      onUserGuideClick,
      showUserGuide,
      userGuideBtnRef,
      userGuideDivRef,
    };
  },
};
</script>

<style scoped lang="scss">
.user-guide {
  padding: 10px;
}
.header {
  font-weight: bold;
}

ul,
li,
p,
div {
  margin: 0;
}

.theme-dark .bg-highlight {
  background-color: #747474;
}

.theme-light .bg-highlight {
  background-color: #e7e6e6;
}
</style>
