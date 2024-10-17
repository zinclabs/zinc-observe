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

<q-table
  :title="t('ticket.header')"
  :rows="tickets"
  row-key="id"
  :pagination="pagination"
  :filter="filterQuery"
  :filter-method="filterData"
  @row-click="editTicket"
>
    <template #top-right>
    <q-input
        v-model="filterQuery"
        filled
        borderless
        dense
        :placeholder="t('ticket.search')"
    >
        <template #append>
        <q-icon name="search" class="cursor-pointer" />
        </template>
    </q-input>
    <q-btn
        class="q-ml-sm"
        color="primary"
        icon="add"
        :label="t(`ticket.add`)"
        @click="addTicket"
    />
    </template>

    <template v-slot:body-cell-#="props">
    <q-td :props="props" width="80" @click="editTicket(props)">
        {{ props.value }}
    </q-td>
    </template>
</q-table>
