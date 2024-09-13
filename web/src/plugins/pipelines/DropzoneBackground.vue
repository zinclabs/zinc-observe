<!-- Copyright 2023 Zinc Labs Inc.

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


<script lang="ts" setup>
import { Background } from "@vue-flow/background";
</script>

<template>
  <div class="dropzone-background">
    <Background :size="2"
:gap="20" pattern-color="#BDBDBD" />

    <div class="overlay">
      <slot />
    </div>
  </div>
</template>
