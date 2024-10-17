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
  <q-item
    :data-test="`menu-link-${link}-item`"
    v-ripple="true"
    :to="
      !external
        ? {
            path: link,
            exact: false,
            query: {
              org_identifier: store.state.selectedOrganization?.identifier,
            },
          }
        : ''
    "
    clickable
    :class="{
      'ql-item-mini': mini,
      'q-router-link--active':
        router.currentRoute.value.path.indexOf(link) == 0 && link != '/',
      'q-link-function': title == 'Functions',
    }"
    :target="target"
    @click="external ? openWebPage(link) : ''"
  >
    <q-item-section v-if="icon" avatar>
      <q-icon :name="icon" />
    </q-item-section>
    <q-item-section v-else-if="iconComponent" avatar>
      <q-icon><component :is="iconComponent" /></q-icon>
    </q-item-section>

    <q-item-section>
      {{ title }}
    </q-item-section>
  </q-item>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";

export default defineComponent({
  name: "MenuLink",
  props: {
    title: {
      type: String,
      required: true,
    },

    caption: {
      type: String,
      default: "",
    },

    link: {
      type: String,
      default: "#",
    },

    icon: {
      type: String,
      default: "",
    },

    iconComponent: {
      type: Object,
      default: () => ({}),
    },

    mini: {
      type: Boolean,
      default: true,
    },

    target: {
      type: String,
      default: "_self",
    },

    external: {
      type: Boolean,
      default: false,
    },
  },
  setup() {
    const store = useStore();
    const router: any = useRouter();
    const openWebPage = (url: string) => {
      window.open(url, "_blank");
    };

    return {
      store,
      router,
      openWebPage,
    };
  },
});
</script>

<style scoped lang="scss">
.q-item {
  padding: 3px 8px;
  margin: 0 8px;
  border-radius: 6px;

  /* Overriding default height */
  min-height: 30px;

  &.q-router-link--active {
    background-color: $primary;
    color: white;

    &::before {
      content: " ";
      width: 10px;
      height: 40px;
      position: absolute;
      left: -30px;
      top: 0;
      background-color: inherit;
      border-radius: 6px;
    }
  }

  &.ql-item-mini {
    margin: 0;

    &::before {
      display: none;
    }
  }
}

.q-item__section--avatar {
  margin: 0;
  padding: 0;
  min-width: 40px;
}
</style>
