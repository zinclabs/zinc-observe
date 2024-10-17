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
  <div id="app">Upload file</div>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";

export default defineComponent({
  setup() {
    const allFiles = ref([]);

    const documentList: any = ref([
      {
        projectSub: {
          projectSubId: null,
        },
        fileBean: [],
      },
    ]);

    const addFiles = (files: []) => {
      if (files.length > 0) {
        allFiles.value.push(...files);
      }
    };

    const removeFiles = (files: any[]) => {
      if (files.length > 0) {
        files.forEach((v) => {
          const idx = allFiles.value.findIndex((f: any) => f.name === v.name);
          idx > -1 && allFiles.value.splice(idx, 1);
        });
      }
    };

    const upload = (projectSubId: any) => {
      // const finalFormData = new FormData();
      documentList.value[0].projectSub.projectSubId = projectSubId;
      const formData: any = new FormData();
      allFiles.value.forEach((file) => {
        formData.append("fileBean", file);
      });
      documentList.value[0].fileBean.push(formData);

      for (var pair of formData.entries())
        console.log(pair[0] + ", " + pair[1]);

      // Axios POST request here...
    };

    return {
      addFiles,
      removeFiles,
      upload,
    };
  },
});
</script>
