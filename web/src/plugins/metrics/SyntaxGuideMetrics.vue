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

<template>
  <q-btn
    data-cy="syntax-guide-button"
    size="sm"
    dense
    flat
    class="syntax-guide-button"
    :class="sqlmode ? 'sql-mode' : 'normal-mode'"
    :label="t('search.syntaxGuideLabel')"
    icon="help"
  >
    <q-menu :class="store.state.theme == 'dark' ? 'theme-dark' : 'theme-light'">
      <q-card flat v-if="!sqlmode">
        <q-card-section class="syntax-guide-title">
          <div class="label">{{ t("search.syntaxGuideLabel") }}</div>
        </q-card-section>
        <q-separator />
        <q-card-section class="q-pt-none answers">
          <div class="syntax-section">
            <div class="syntax-guide-text">
              <ul class="guide-list">
                <li>
                  For instant vector selectors, use
                  <span class="bg-highlight"
                    >metric_name{label1="value1", label2="value2"}</span
                  >
                </li>
                <li>
                  For range vector selectors, use
                  <span class="bg-highlight">metric_name[5m]</span>
                </li>
                <li>
                  To aggregate data, use
                  <span class="bg-highlight">sum by (label)(metric_name)</span>
                  or
                  <span class="bg-highlight"> avg by (label)(metric_name)</span>
                </li>
                <li>
                  For rate calculations over a range vector, use
                  <span class="bg-highlight">rate(metric_name[5m])</span>
                </li>
              </ul>
            </div>
          </div>
        </q-card-section>
      </q-card>
      <q-card flat v-else>
        <q-card-section class="syntax-guide-title">
          <div class="label">Syntax Guide: SQL Mode</div>
        </q-card-section>
        <q-separator />
        <q-card-section class="q-pt-none answers">
          <div class="syntax-section">
            <div class="syntax-guide-text">
              <ul class="guide-list">
                <li>
                  For full text search of value 'error' use
                  <span class="bg-highlight"
                    >SELECT * FROM <b>stream</b> WHERE
                    match_all_raw('error')</span
                  >
                </li>
                <li>
                  For case-insensitive full text search of value 'error' use
                  <span class="bg-highlight"
                    >SELECT * FROM <b>stream</b> WHERE
                    match_all_raw_ignore_case('error')</span
                  >
                </li>
                <li>
                  For column search of value 'error' use
                  <span class="bg-highlight"
                    >SELECT * FROM <b>stream</b> WHERE
                    str_match(<b>fieldname</b>, 'error')</span
                  >
                </li>
                <li>
                  To search value 200 for code column use
                  <span class="bg-highlight"
                    >SELECT * FROM <b>stream</b> WHERE code=200</span
                  >
                </li>
                <li>
                  To search value 'stderr' for stream column use
                  <span class="bg-highlight"
                    >SELECT * FROM <b>stream</b> WHERE stream='stderr'</span
                  >
                </li>
                <li>
                  To search and use query function <i>extract_ip</i> on column
                  log use
                  <span class="bg-highlight"
                    >SELECT extract_ip(log) FROM <b>stream</b> WHERE
                    code=200</span
                  >
                </li>
                <li>
                  For additional examples,
                  <a
                    href="https://openobserve.ai/docs/example-queries/"
                    target="_blank"
                    class="hover:tw-underline text-primary"
                    >click here</a
                  >.
                </li>
              </ul>
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
  name: "SyntaxGuideMetrics",
  props: {
    sqlmode: {
      type: Boolean,
      default: false,
    },
  },
  setup() {
    const { t } = useI18n();
    const store = useStore();
    return {
      t,
      store,
    };
  },
});
</script>

<style lang="scss" scoped>
.guide-list {
  padding: 0 10px;
  margin: 10px 0 0 0;
}
.guide-list li {
  font-size: 14px;
  line-height: 23px;
}
.q-btn:before {
  border: 0px solid #d5d5d5;
}

.syntax-guide-button {
  cursor: pointer;
  text-transform: capitalize;
  padding: 5px 5px;
  height: 30px;
  font-weight: bold;
  border: 1px solid rgba(89, 96, 178, 0.3);
}

.normal-mode {
  background-color: rgba(0, 0, 0, 0.05);
}

.sql-mode {
  background-color: rgba(89, 96, 178, 0.8);
  color: #ffffff;
}

.syntax-guide-title {
  width: 420px;

  .label {
    font-size: 15px;
    font-weight: bold;
  }
}

.syntax-guide-sub-title {
  color: $primary;
  font-size: 15px;
  margin-left: 5px;
}

.syntax-guide-text {
  font-size: 12px;
  margin-left: 5px;
}

.syntax-section {
  margin-bottom: 5px;
}
.bg-highlight {
  padding-left: 5px;
  padding-right: 5px;
}

.theme-dark .bg-highlight {
  background-color: #747474;
}

.theme-light .bg-highlight {
  background-color: #e7e6e6;
}
</style>
