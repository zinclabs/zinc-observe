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
  <q-card class="column full-height">
    <q-card-section class="q-px-md q-py-md text-black">
      <div class="row items-center no-wrap">
        <div class="col">
          <div v-if="beingUpdated" class="text-body1 text-bold text-dark">
            {{ t("ticket.updateTicket") }}
          </div>
          <div v-else class="text-body1 text-bold text-dark">
            {{ t("ticket.createTicket") }}
          </div>
        </div>
        <div class="col-auto">
          <q-btn v-close-popup round flat icon="cancel" />
        </div>
      </div>
    </q-card-section>
    <q-separator />
    <q-form ref="addTicketForm" @submit="onSubmit">
      <q-card-section class="q-w-md q-mx-lg">
        <q-input
          v-if="beingUpdated"
          v-model="ticketData.id"
          :readonly="beingUpdated"
          :disabled="beingUpdated"
          :label="t('ticket.id')"
        />

        <q-input
          v-model="ticketData.subject"
          :placeholder="t('ticket.inputTextHolder')"
          :label="t('ticket.subject')"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
        />

        <q-input
          v-model="ticketData.description"
          :placeholder="t('ticket.inputTextHolder')"
          :label="t('ticket.description')"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
        />

        <q-select
          v-if="beingUpdated"
          v-model="ticketData.status"
          :options="status"
          :label="t('ticket.status')"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          readonly
          disabled
          filled
          dense
        />

        <q-input
          v-model="ticketData.comments"
          :placeholder="t('ticket.inputTextHolder')"
          :label="t('ticket.comments')"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
        />
        <!-- type="textarea" -->

        <q-uploader
          v-slot:spinner="scope"
          ref="fileUploader"
          counter
          :label="t('common.attachments')"
          multiple
          max-files="3"
          color="white"
          text-color="dark"
          flat
          :factory="uploadAttachment"
          @rejected="onRejected"
        >
          <template>
            <q-spinner v-if="scope.isUploading" class="q-uploader__spinner">
            </q-spinner>
          </template>
        </q-uploader>

        <div class="flex justify-center q-mt-xl">
          <q-btn
            v-close-popup
            class="q-mb-md text-bold no-border"
            :label="t('ticket.cancel')"
            text-color="light-text"
            padding="sm md"
            color="accent"
            no-caps
          />
          <q-btn
            :label="beingUpdated ? t('ticket.update') : t('ticket.save')"
            class="q-mb-md text-bold no-border q-ml-md"
            color="secondary"
            padding="sm xl"
            type="submit"
            no-caps
          />
        </div>
      </q-card-section>
    </q-form>
  </q-card>
</template>

<script lang="ts">
// @ts-nocheck
import { defineComponent, ref } from "vue";
import ticketService from "../../services/tickets";
import attachmentService from "../../services/attachments";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { getImageURL } from "../../utils/zincutils";

const defaultValue = () => {
  return {
    id: "",
    user_id: "",
    subject: "",
    description: "",
    created_at: "",
    updated_at: "",
    status: "",
    comments: "",
    attachments: "",
    files: "",
  };
};

var callTicket;

export default defineComponent({
  name: "ComponentAddUpdateUser",
  props: {
    modelValue: {
      type: Object,
      default: () => defaultValue(),
    },
  },
  emits: [
    "update:modelValue",
    "fileUploaded",
    "updated",
    "finish",
    "isUploading",
  ],
  setup() {
    const store: any = useStore();
    const beingUpdated: any = ref(false);
    const isUploading: any = ref(false);
    const status: any = ref(["Created", "In-Progress", "Completed"]);
    const addTicketForm: any = ref(null);
    const disableColor: any = ref("");
    const ticketData: any = ref(defaultValue());
    const { t } = useI18n();

    return {
      t,
      disableColor,
      beingUpdated,
      status,
      ticketData,
      addTicketForm,
      store,
      isUploading,
      getImageURL,
    };
  },
  created() {
    if (this.modelValue && this.modelValue.id) {
      this.beingUpdated = true;
      this.disableColor = "grey-5";
      this.ticketData = {
        id: this.modelValue.id,
        subject: this.modelValue.subject,
        description: this.modelValue.description,
        created_at: this.modelValue.created_at,
        updated_at: this.modelValue.updated_at,
        status: this.modelValue.status,
        comments: this.modelValue.comments,
        attachments: this.modelValue.attachments,
      };
    }
  },
  methods: {
    validateTicketSubject(data: string | any[]) {
      if (data.length < 3) {
        return "User name must be at least 3 characters long";
      }
    },
    validateTicketDescription(data: string | any[]) {
      if (data.length < 3) {
        return "You must select a role";
      }
    },
    validateTicketComments(data: string | any[]) {
      if (data.length < 3) {
        return "You must select a role";
      }
    },
    uploadAttachment(files: any, updateFileStatus: any) {
      this.$emit("isUploading", true);
      var that = this;
      const uploader = this.$refs.fileUploader;
      attachmentService
        .getPresignedUrl(files[0].name, files[0].type)
        .then((response) => {
          attachmentService
            .upload(response.data.data, files[0])
            .then((response) => {
              this.$emit("finish");
              this.$emit("fileUploaded", true);
              this.$refs.fileUploader.updateFileStatus(files[0], "uploaded", 1);
              this.isUploading = false;
              files[0].__status = "uploaded";
            });
        });
    },
    onRejected(rejectedEntries: string | any[]) {
      this.$q.notify({
        type: "negative",
        message: `${rejectedEntries.length} file(s) did not pass validation constraints`,
      });
    },
    onSubmit() {
      const dismiss = this.$q.notify({
        spinner: true,
        message: "Please wait...",
        timeout: 2000,
      });
      this.addTicketForm.validate().then((valid: any) => {
        if (!valid) {
          return false;
        }
        // console.log("Form is valid");
        this.ticketData.user_id = this.store.state.userInfo.user_id;

        delete this.ticketData.created_at;
        delete this.ticketData.updated_at;
        delete this.ticketData.user_id;
        delete this.ticketData.attachments;
        delete this.ticketData.files;
        delete this.ticketData.comments;

        this.ticketData.requester_id = 88000825587;
        this.ticketData.responder_id = 88000825587;
        this.ticketData.email = this.store.state.userInfo.name;
        this.ticketData.type = "Incident";
        this.ticketData.priority = 1;
        this.ticketData.source = 1;
        this.ticketData.status = 2;

        var ticketId = this.ticketData.id;
        delete this.ticketData.id;

        if (ticketId == "") {
          callTicket = ticketService.create(this.ticketData);
        } else {
          callTicket = ticketService.update(ticketId, this.ticketData);
        }

        callTicket.then((res) => {
          var data = res.data;
          this.ticketData = {
            id: "",
            user_id: "",
            subject: "",
            description: "",
            created_at: "",
            updated_at: "",
            status: "",
            comments: "",
            attachments: "",
          };

          this.$emit("update:modelValue", data);
          this.$emit("updated", data);
          this.addTicketForm.resetValidation();
          dismiss();
        });
      });
    },
  },
});
</script>

<style lang="scss">
.q-uploader {
  border: 1px dashed $input-border;
  margin-top: 3rem;
  width: 100%;

  &__header {
    &-content {
      margin-top: -2rem;
      .q-uploader {
        &__title {
          margin-bottom: 1.8rem;
        }
        &__subtitle {
          transform: translateY(-0.5rem);
          padding-left: 0.375rem;
        }
      }
      .q-btn {
        align-self: flex-end;
        margin-right: 0.375rem;
      }
    }
    & ~ .q-uploader {
      &__list:empty {
        display: none;
      }
    }
  }
}
</style>
