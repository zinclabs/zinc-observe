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
    v-if="!childrens.length"
    :data-test="`menu-link-${link}-item`"
    v-ripple="true"
    dense
    :to="
      !external
        ? {
            path: link,
            exact: false,
            query: {
              org_identifier: store?.state?.selectedOrganization?.identifier,
            },
          }
        : ''
    "
    class="menu-btn"
    :class="{
      'q-router-link--active':
        router.currentRoute.value.path.indexOf(link) == 0 && link != '/',
      'q-link-function': title == 'Functions',
    }"
    :target="target"
    @click="external ? openWebPage(link) : ''"
    >{{ title }}
  </q-btn>
  <q-btn v-else dense flat class="menu-btn"
:label="title">
    <q-menu
      fit
      anchor="bottom right"
      self="top right"
      transition-show="flip-right"
      transition-hide="flip-left"
    >
      <q-list>
        <q-item
          v-for="(child, index) in childrens"
          v-ripple="true"
          v-close-popup="true"
          :data-test="`menu-link-${child.link}-item`"
          clickable
          :to="
            !child.external
              ? {
                  path: child.link,
                  exact: false,
                  query: {
                    org_identifier:
                      store.state.selectedOrganization?.identifier,
                  },
                }
              : ''
          "
          :target="child.target"
          @click="child.external ? openWebPage(child.link) : ''"
        >
          <q-item-section>
            <q-item-label>{{ child.title }}</q-item-label>
          </q-item-section>

          <q-item-section
            v-if="child.pin"
            :data-test="`menu-link-${child.link}-pin-item`"
            side
          >
            <q-icon name="push_pin" color="grey"
size="xs" />
          </q-item-section>
        </q-item>
      </q-list>
    </q-menu>
  </q-btn>
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

    childrens: {
      type: Array,
      default: () => [],
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
.menu-btn {
  color: $text-color;
  margin-right: 5px;
  font-size: var(--menu-font-size);
  padding: 0px 5px;
  text-transform: capitalize;

  &::before {
    border: 0px;
  }

  &.q-router-link--active {
    background-color: $primary;
    color: $white;
  }
}

.dark-mode {
  .menu-btn {
    color: $white;
  }
}
</style>
