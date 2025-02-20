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

/* eslint-disable @typescript-eslint/no-explicit-any */
import http from "./http";

const billings = {
  get_quota_threshold: (org_identifier: string) => {
    return http().get(`/api/${org_identifier}/billings/quota_threshold`);
  },
  create_payment_subscribe: (org_identifier: string, data: object) => {
    return http().post(`/api/${org_identifier}/billings/payment_source`, data);
  },
  unsubscribe: (org_identifier: string) => {
    return http().get(`/api/${org_identifier}/billings/unsubscribe`);
  },
  list_subscription: (org_identifier: string) => {
    return http().get(`/api/${org_identifier}/billings/list_subscription`);
  },
  list_paymentsources: (org_identifier: string) => {
    return http().get(`/api/${org_identifier}/billings/list_paymentsource`);
  },
  resume_subscription: (org_identifier: string) => {
    return http().get(`/api/${org_identifier}/billings/resume_subscription`);
  },
  get_hosted_url: (org_identifier: string, plan_name: string) => {
    return http().get(
      `/api/${org_identifier}/billings/hosted_subscription_url?plan=${plan_name}`
    );
  },
  get_session_url: (org_identifier: string, customer_id: string) => {
    return http().get(
      `/api/${org_identifier}/billings/billing_portal?customer_id=${customer_id}`
    );
  },
  retrieve_hosted_page: (org_identifier: string, hosted_page_id: string) => {
    return http().get(
      `/api/${org_identifier}/billings/hosted_page_status/${hosted_page_id}`
    );
  },
  change_payment_detail: (org_identifier: string, hosted_page_id: string) => {
    return http().get(
      `/api/${org_identifier}/billings/change_payment_detail/${hosted_page_id}`
    );
  },
  list_invoice_history: (org_identifier: string) => {
    return http().get(`/api/${org_identifier}/billings/invoices`);
  },
  get_data_usage: (org_identifier: string, usage_date: string) => {
    return http().get(
      `/api/${org_identifier}/billings/data_usage/${usage_date}`
    );
  },
};

export default billings;
