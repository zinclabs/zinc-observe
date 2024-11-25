// Copyright 2023 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

import { useQuasar } from "quasar";
import { useStore } from "vuex";
import TransformService from "@/services/jstransform";

const useFunctions = () => {
  const store = useStore();
  const $q = useQuasar();

  const getAllFunctions = async () => {
    try {
      return await TransformService.list(
        1,
        100000,
        "name",
        false,
        "",
        store.state.selectedOrganization.identifier
      )
        .then((res: any) => {
          store.dispatch("setFunctions", res.data.list);
          return;
        })
        .catch((e: any) => {
          console.log(e,"error")
          throw new Error(e.message);
        });
    } catch (e: any) {
      console.log(e,"error")
      throw new Error(e.message);
    }
  };

  return { getAllFunctions };
};

export default useFunctions;
