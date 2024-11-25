import { createStore } from "vuex";

const store = createStore({
  state: {
    API_ENDPOINT: "http://localhost:5080",
    selectedOrganization: {
      label: "default Organization",
      id: 159,
      identifier: "default_organization_01",
      user_email: "example@gmail.com",
      subscription_type: "",
      activeTab:"custom",
    },
    currentuser: {
      role: "",
    },
    userInfo: {
      at_hash: "QicVZWM5kDY6hOzf",
      email_verified: false,
      given_name: "example",
      picture: "",
      aud: "31ds0mr4psua0p58353l3t6j61",
      token_use: "id",
      auth_time: 1678689752,
      name: "example",
      exp: 1678776152,
      iat: 1678689753,
      family_name: "example",
      email: "example@gmail.com",
    },
    zoConfig: {
      version: "v0.2.0",
      commit_hash: "dc2b38c0f8be27bde395922d61134f09a3b4c",
      build_date: "2023-03-11T03:55:28Z",
      default_fts_keys: ["log", "message", "msg", "content", "data"],
      show_stream_stats_doc_num: true,
      default_functions: [
        {
          name: "match_all_raw",
          text: "match_all_raw('v')",
        },
        {
          name: "match_all_raw_ignore_case",
          text: "match_all_raw_ignore_case('v')",
        },
        {
          name: "match_all",
          text: "match_all('v')",
        },
        {
          name: "str_match",
          text: "str_match(field, 'v')",
        },
        {
          name: "str_match_ignore_case",
          text: "str_match_ignore_case(field, 'v')",
        },
        {
          name: "re_match",
          text: "re_match(field, 'pattern')",
        },
        {
          name: "re_not_match",
          text: "re_not_match(field, 'pattern')",
        },
      ],
    },
    organizationData: {
      organizationPasscode: "",
      allDashboardList: {},
      rumToken: {
        rum_token: "",
      },
      quotaThresholdMsg: "",
      functions: [],
      streams: {},
      folders: [],
      organizationSettings: {
        scrape_interval: 15,
        trace_id_field_name: "trace_id",
        span_id_field_name: "span_id",
      },
      isDataIngested: false,
    },
  },
});

export default store;
