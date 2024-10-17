<!-- Copyright 2023 OpenObserve Inc.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
-->

<!-- eslint-disable vue/v-on-event-hyphenation -->
<template>
  <div
    :class="position === 'bottom' ? 'q-py-sm' : 'q-pt-sm'"
    class="q-table__control full-width row justify-between"
  >
    <div
      v-if="position === 'bottom' && maxRecords"
      class="max-result"
      style="justify-content: start"
    >
      <span class="q-table__bottom-item">{{ t("search.maxRecords") }}</span>
      <q-input
        v-model="maxRecords"
        filled
        dense
        class="max-records-input"
        @blur="changeMaxRecordToReturn"
      />
    </div>
    <div
      v-if="position === 'top' && pageTitle"
      class="text-bold row items-center"
    >
      <q-btn
        v-if="
          collapsibleIcon === 'show' &&
          searchCollapseImage == 'collapse_sidebar_icon'
        "
        :icon="'img:' + getImageURL('images/common/collapse_sidebar_icon.svg')"
        class="q-mr-sm"
        size="sm"
        round
        flat
        @click="toggleSidePanel"
      />
      <q-btn
        v-if="
          collapsibleIcon === 'show' &&
          searchCollapseImage == 'expand_sidebar_icon'
        "
        :icon="'img:' + getImageURL('images/common/expand_sidebar_icon.svg')"
        class="q-mr-sm"
        size="sm"
        round
        flat
        @click="toggleSidePanel"
      />
      <div class="q-ml-xs">
        {{ resultTotal }}
        {{ pageTitle.slice(-1) == "s" ? pageTitle.slice(0, -1) : pageTitle }}(s)
      </div>
    </div>
    <div class="q-table__control q-ml-auto">
      <span class="q-table__bottom-item">
        {{ t("search.showing") }}
        {{ (scope.pagination.page - 1) * scope.pagination.rowsPerPage + 1 }} -
        {{
          scope.pagination.page * scope.pagination.rowsPerPage >= resultTotal
            ? resultTotal
            : scope.pagination.page * scope.pagination.rowsPerPage
        }}
        {{ t("search.of") }} {{ resultTotal }}

        <!-- {{ (scope.pagination.page - 1) * scope.pagination.rowsPerPage + 1 }}-{{
          scope.pagination.page * scope.pagination.rowsPerPage
        }}
        of {{ resultTotal }} -->
      </span>

      <div v-if="position === 'bottom'" class="flex items-center">
        <q-separator vertical inset class="q-mr-md" />

        <span class="q-table__bottom-item">
          {{ t("search.recordsPerPage") }}
        </span>
        <q-select
          v-model="scope.pagination.rowsPerPage"
          class="q-mr-md"
          borderless
          size="sm"
          dense
          :options="perPageOptions"
          @update:modelValue="changePagination"
        />
      </div>

      <q-btn-group>
        <q-btn
          icon="chevron_left"
          :text-color="scope.isFirstPage ? '$light-text2' : '$dark'"
          class="pageNav"
          color="#FAFBFD"
          size="sm"
          flat
          :disable="scope.isFirstPage"
          @click="scope.prevPage"
        />
        <q-separator vertical />
        <q-btn
          icon="chevron_right"
          :text-color="scope.isLastPage ? '$light-text2' : '$dark'"
          class="pageNav"
          color="#FAFBFD"
          size="sm"
          flat
          :disable="scope.isLastPage"
          @click="scope.nextPage"
        />
      </q-btn-group>
    </div>
  </div>
</template>

<script lang="ts">
// @ts-nocheck
import { defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { getImageURL } from "../../../utils/zincutils";

export default defineComponent({
  name: "QTablePagination",
  // eslint-disable-next-line vue/require-prop-types
  props: [
    "scope",
    "pageTitle",
    "resultTotal",
    "maxRecordToReturn",
    "changeRecordPerPage",
    "perPageOptions",
    "position",
    "collapsibleIcon",
  ] as string[],
  emits: ["update", "update:maxRecordToReturn", "update:changeRecordPerPage"],
  setup(props, { emit }) {
    const { t } = useI18n();
    const router = useRouter();
    const maxRecords = ref(props.maxRecordToReturn);
    const store = useStore();
    const searchCollapseImage: any = ref("collapse_sidebar_icon");

    const changePagination = (val: any) => {
      emit("update:changeRecordPerPage", val);
    };

    const changeMaxRecordToReturn = (e: { target: { value: any } }) => {
      emit("update:maxRecordToReturn", e.target.value);
    };

    const toggleSidePanel = () => {
      store.dispatch(
        "setSearchCollapseToggle",
        store.state.searchCollapsibleSection == 0 ? 20 : 0
      );
    };

    return {
      t,
      store,
      router,
      maxRecords,
      toggleSidePanel,
      searchCollapseImage,
      changePagination,
      changeMaxRecordToReturn,
      getImageURL,
    };
  },
  computed: {
    sidebarIcon() {
      return this.store.state.searchCollapsibleSection;
    },
  },
  watch: {
    sidebarIcon(newVal: any, oldVal: any) {
      if (newVal != oldVal && this.router.currentRoute.value.name == "logs") {
        if (this.store.state.searchCollapsibleSection == 0) {
          this.searchCollapseImage = "expand_sidebar_icon";
        } else {
          this.searchCollapseImage = "collapse_sidebar_icon";
        }
      }
    },
  },
});
</script>

<style lang="scss">
.footer-text {
  margin-right: 1rem;
  font-size: 0.75rem;
  font-weight: 600;
}
.q-table__bottom-item {
  @extend .footer-text;
}
.q-select .q-field {
  &__native {
    @extend .footer-text;
    text-align: center;
    margin-right: 0;
  }
  &__append {
    padding: 0;
  }
}

.pageNav {
  padding: 0.125rem 0.5rem;
}

.max-result {
  justify-content: center;
  align-items: center;
  white-space: nowrap;
  display: flex;
  width: 200px;

  .q-field {
    max-width: 3.5rem;
  }

  .max-records-input {
    .q-field {
      &__control {
        // background-color: #fafbfd !important;
        max-width: 2.5rem;
        height: 1.5rem;
        padding: 0;
      }
      &__native {
        font-size: 0.75rem;
        text-align: center;
        font-weight: 600;
        // color: $dark;
        padding: 0;
        width: fit-content;
      }
    }
  }
}
</style>
