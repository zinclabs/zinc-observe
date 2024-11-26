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

import { flushPromises, mount } from '@vue/test-utils';
import { describe, it, expect, vi, beforeEach,afterEach } from 'vitest';
import AddAlert from '@/components/alerts/AddAlert.vue';
import ScheduledAlert from '@/components/alerts/ScheduledAlert.vue';
import VariablesInput from '@/components/alerts/VariablesInput.vue';
import FieldsInput from '@/components/alerts/FieldsInput.vue';
import alertsService from "@/services/alerts";
import { Dialog, Notify } from 'quasar';
import store from '../../helpers/store';
import { createI18n } from 'vue-i18n';
import axios, { AxiosResponse } from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { installQuasar } from '../../helpers';
import router from "../../helpers/router";

import PreviewAlert from '@/components/alerts/PreviewAlert.vue';

import i18n from '@/locales'

import QueryEditor from '@/components/QueryEditor.vue';

import { useLocalOrganization } from '@/utils/zincutils';

vi.spyOn(global, 'fetch');


installQuasar({
  plugins: [Dialog, Notify ],
});



// Create an i18n instance with the mock message
// Create a mock for axios
const mock = new MockAdapter(axios, { delayResponse: 0 });

// Mock API response
mock
  .onGet('http://localhost:5080/api/default_organization_01/functions?page_num=1&page_size=100000&sort_by=name&desc=false&name=')
  .reply(200, {
    data: {
      list: [
        {
          function: '.a=123 \n .',
          name: 'lskjf',
          params: 'row',
          numArgs: 1,
          transType: 0,
        },
        {
          function: '.a=123 \n .',
          name: 'nginx_function',
          params: 'row',
          numArgs: 1,
          transType: 0,
        },
      ],
    },
  });





describe('AddAlert Component', () => {
  let wrapper: any;



  // Ensure async operations are complete
  beforeEach (async () => {


    const updateStreamsMock = vi.fn().mockResolvedValue({ success: true });


    
    vi.useFakeTimers();

    wrapper = mount(AddAlert, {
      global: {
        provide: {
          store: store,
        },
        plugins: [i18n, router],
      },
      props: {
        title: 'Add Alert',
      },
      mocks:{
        updateStreams: updateStreamsMock
      }
    });
  });

  

  it('renders the title passed as a prop', () => {
    const titleElement = wrapper.find('[data-test="add-alert-title"]');
    expect(titleElement.exists()).toBe(true);
    expect(titleElement.text()).toBe('Add Alert');
  });

  it('renders the name input', () => {
    const nameInput = wrapper.find('[data-test="add-alert-name-input"]');
    expect(nameInput.exists()).toBeTruthy();
  });

  it('renders the stream input', () => {
    const streamSelect = wrapper.find('[data-test="add-alert-stream-select"]');
    expect(streamSelect.exists()).toBeTruthy();
  });

  it('defaults to Scheduled type alerts', () => {
    const scheduledRadio = wrapper.find('[data-test="add-alert-scheduled-alert-radio"]');
    expect(scheduledRadio.find('.q-radio__inner--truthy').exists()).toBeTruthy();
  });

  it('renders conditions section', () => {
    expect(wrapper.find('[data-test="alert-conditions-text"]').text()).toBe(
      'Conditions * (AND operator is used by default to evaluate multiple conditions)'
    );
    expect(wrapper.find('[data-test="alert-conditions-select-column"]').exists()).toBeTruthy();
    expect(wrapper.find('[data-test="alert-conditions-operator-select"]').exists()).toBeTruthy();
    expect(wrapper.find('[data-test="alert-conditions-value-input"]').exists()).toBeTruthy();
  });

  it('renders duration input', () => {
    expect(wrapper.find('[data-test="scheduled-alert-period-title"]').text()).toBe(
      'Period *'
    );
    expect(wrapper.find('[data-test="scheduled-alert-period-input"]').exists()).toBeTruthy();
    expect(wrapper.find('[data-test="scheduled-alert-period-unit"]').exists()).toBeTruthy();
  });

  it('renders frequency input', () => {
    expect(wrapper.find('[data-test="scheduled-alert-frequency-title"]').text()).toBe(
      'Frequency *'
    );
    expect(wrapper.find('[data-test="scheduled-alert-frequency-input"]').exists()).toBeTruthy();
    expect(wrapper.find('[data-test="scheduled-alert-frequency-unit"]').exists()).toBeTruthy();
  });

  it('renders destination select input', () => {
    expect(wrapper.find('[data-test="add-alert-destination-select"]').exists()).toBeTruthy();
  });

  it('renders ScheduledAlert component', () => {
     
    const scheduledAlertComponent = wrapper.findComponent(ScheduledAlert);

    expect(scheduledAlertComponent.exists()).toBe(true);

    const FieldsInputComponent =  wrapper.findComponent(FieldsInput);
    expect(FieldsInputComponent.exists()).toBe(true);
  });

  

  describe("When user fills form and clicks submit",async () => {

    

    
    // const mockStreamName = "k8s_json";
    // const mockAlertName = "alert2";


    beforeEach(async () => {
      

      mock
        .onPost("http://localhost:5080/api/default_organization_01/k8s_json/alerts?type=logs")
        .reply((config) => {  

          const data = JSON.parse(config.data);
          if(data.destinations.length === 0){
            return [400, {

              data:{
                message: "Destination Required",
                code: 400,
              }
            }];
          }


          return [200, {

            data:{
              message: "Alert created successfully",
              code: 200,
            }
          }];
        });
      mock
        .onGet("http://localhost:5080/api/default_organization_01/streams?type=logs")
        .reply((config) => {          
          return [200, {
            data:{
              message: "",
              code: 200,
            }
          }];
        });
  

  
    //   // // Fill form fields
      // const alertNameWrapper = wrapper.find('[data-test="add-alert-name-input"]');
      // await alertNameWrapper.find("input").setValue(mockAlertName);

  
      // wrapper.vm.formData.stream_name = mockStreamName;
      // wrapper.vm.formData.stream_type = "logs";
      // // // wrapper.vm.formData.query_condition.conditions[0].column = "kubernetes.container_hash";
      // // // // wrapper.vm.formData.query_condition.conditions[0].operator = "=";
      // // // console.log(wrapper.vm.formData.query_condition.conditions[0],'cond')
      // // // // wrapper.vm.formData.query_condition.conditions[0].value = 123;
      // wrapper.vm.formData.aggregation = null;

      // wrapper.vm.formData.is_real_time = false

      // wrapper.vm.formData.lastEditedBy =  ""
      // wrapper.vm.formData.lastTriggeredAt = 0;

      // wrapper.vm.formData.trigger_condition.period = 1;
      // wrapper.vm.formData.trigger_condition.frequency = 1;
      // wrapper.vm.formData.trigger_condition.silence = 10;
  
      // wrapper.vm.formData.destinations[0] = "dest1";
      // wrapper.vm.formData.aggregation = null;
    });
    it('should retain the sql query when the user switches to the SQL tab from custom tab', async () => {
      const sqlTab = wrapper.find('[data-test="scheduled-alert-sql-tab"]');
      const customTab = wrapper.find('[data-test="scheduled-alert-custom-tab"]');
      await sqlTab.trigger('click');
           
      const scheduledAlertComponent = wrapper.findComponent(ScheduledAlert);

      expect(scheduledAlertComponent.exists()).toBe(true);
      await flushPromises()

      const QueryEditorComponent = await wrapper.findComponent(QueryEditor);
      expect(QueryEditorComponent.exists()).toBe(true);
      const sqlQuery = "SELECT code FROM 'default";

      scheduledAlertComponent.vm.updateQueryValue(sqlQuery);
      await customTab.trigger('click');

      await sqlTab.trigger('click');
      expect(scheduledAlertComponent.vm.sql).toBe(sqlQuery);
    });
    it('should submit the form when the user gives all the data for test usage', async () => {
      wrapper.vm.addAlertForm = {
        validate: vi.fn().mockResolvedValue(true), // Simulates a valid form
        resetValidation: vi.fn(), // Mock other methods if needed
      };
      const submitSpy = vi.spyOn(alertsService,'create');
      wrapper.vm.formData = {
        "name": "basic_standard_alert_10",
        "stream_type": "logs",
        "stream_name": "k8s_json",
        "is_real_time": "false",
        "query_condition": {
            "conditions": [],
            "sql": "",
            "promql": "",
            "type": "custom",
            "aggregation": {
                "group_by": [
                    ""
                ],
                "function": "avg",
                "having": {
                    "column": "",
                    "operator": ">=",
                    "value": 1
                }
            },
            "promql_condition": null,
            "vrl_function": null,
            "multi_time_range": []
        },
        "trigger_condition": {
            "period": 10,
            "operator": ">=",
            "frequency": 1,
            "cron": "",
            "threshold": 3,
            "silence": 10,
            "frequency_type": "minutes",
            "timezone": "UTC"
        },
        "destinations": [
            "ksjf"
        ],
        "context_attributes": [],
        "enabled": true,
        "description": "",
        "lastTriggeredAt": 0,
        "createdAt": "",
        "updatedAt": "2024-11-21T08:26:50.907Z",
        "owner": "",
        "lastEditedBy": ""
    }


      
      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click')

      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');
      await flushPromises();
      await vi.advanceTimersByTime(500);

      await flushPromises();

      expect(submitSpy).toHaveBeenCalledTimes(1);

    })

    it('should not call the api when user submits the form with invalid sql query / empty query ', async () => {


      const alertCreateSpy = vi.spyOn(alertsService, 'create');


      const sqlTab = wrapper.find('[data-test="scheduled-alert-sql-tab"]');
      await sqlTab.trigger('click');

      const alertNameWrapper = wrapper.find('[data-test="add-alert-name-input"]');
      await alertNameWrapper.find("input").setValue("alert123");

      const streamTypeSelect = wrapper.find('[data-test="add-alert-stream-type-select"]');
      const streamType = streamTypeSelect.findComponent({ name: 'QSelect' });
      expect(streamType.exists()).toBe(true);

      wrapper.vm.formData.stream_type = 'logs';
      wrapper.vm.formData.stream_name = 'k8s_json';

      // const streamNameSelect = await wrapper.find('[data-test="add-alert-stream-select"]');
      // const streamName = await streamNameSelect.findComponent({ name: 'QSelect' });
      // expect(streamName.exists()).toBe(true);
      
      // await streamName.setValue('k8s_json');
  


  
      const scheduledAlertComponent = await wrapper.findComponent(ScheduledAlert);

      expect(scheduledAlertComponent.exists()).toBe(true);
      flushPromises();
      const sqlQuery = "";
      scheduledAlertComponent.vm.updateQueryValue(sqlQuery);

      wrapper.vm.formData.destinations = ["dest1"];

      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');
      await flushPromises();
      await vi.advanceTimersByTime(500);
      expect(alertCreateSpy).toHaveBeenCalledTimes(0);
      // expect(alertCreateSpy).toHaveBeenCalledTimes(0);


    })

    it('should fill the form with the correct data and api should be called',async () => {
      wrapper.vm.addAlertForm = {
        validate: vi.fn().mockResolvedValue(true), // Simulates a valid form
        resetValidation: vi.fn(), // Mock other methods if needed
      };
      const alertCreateSpy = vi.spyOn(alertsService, 'create');

      vi.spyOn(wrapper.vm, 'updateStreams').mockResolvedValue({ success: true });


      const mockUpdateStreamFields = vi.spyOn(wrapper.vm, 'updateStreamFields');
      mockUpdateStreamFields.mockResolvedValue({ success: true });



      const alertNameWrapper = wrapper.find('[data-test="add-alert-name-input"]');
      await alertNameWrapper.find("input").setValue("alert123");



      const streamTypeSelect = await wrapper.find('[data-test="add-alert-stream-type-select"]');
      const streamType = await streamTypeSelect.findComponent({ name: 'QSelect' });
      expect(streamType.exists()).toBe(true);
      await streamType.setValue('logs');

      await flushPromises();

      const streamNameSelect = await wrapper.find('[data-test="add-alert-stream-select"]');
      const streamName = await streamNameSelect.findComponent({ name: 'QSelect' });
      expect(streamName.exists()).toBe(true);
      await streamName.setValue('k8s_json');
      await flushPromises();

      const destinationSelect  = await wrapper.find('[data-test="add-alert-destination-select"]');
      const destination = await destinationSelect.findComponent({ name: 'QSelect' });
      expect(destination.exists()).toBe(true);
      await destination.setValue('dest1');

      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');
      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();

      expect(alertCreateSpy).toHaveBeenCalledTimes(1);

      await flushPromises();




    })

    it('should not call api when user doesnot give alert name ',async () => {
      const alertCreateSpy = vi.spyOn(alertsService, 'create');

      vi.spyOn(wrapper.vm, 'updateStreams').mockResolvedValue({ success: true });

      const mockUpdateStreamFields = vi.spyOn(wrapper.vm, 'updateStreamFields');
      const mockRemoveField = vi.fn();
      wrapper.vm.removeField = mockRemoveField;
      mockUpdateStreamFields.mockResolvedValue({ success: true })


      const streamTypeSelect = await wrapper.find('[data-test="add-alert-stream-type-select"]');
      const streamType = await streamTypeSelect.findComponent({ name: 'QSelect' });
      expect(streamType.exists()).toBe(true);
      await streamType.setValue('logs');

      await flushPromises();
      const scheduledAlertComponent = await wrapper.findComponent(ScheduledAlert);

      expect(scheduledAlertComponent.exists()).toBe(true);
      flushPromises();

      const fieldsInput = await wrapper.findComponent(FieldsInput);
      expect(fieldsInput.exists()).toBe(true);

      wrapper.vm.formData.query_condition.conditions = [];


      const streamNameSelect = await wrapper.find('[data-test="add-alert-stream-select"]');
      const streamName = await streamNameSelect.findComponent({ name: 'QSelect' });
      expect(streamName.exists()).toBe(true);
      await streamName.setValue('k8s_json');
      await flushPromises();

      const destinationSelect  = await wrapper.find('[data-test="add-alert-destination-select"]');
      const destination = await destinationSelect.findComponent({ name: 'QSelect' });
      expect(destination.exists()).toBe(true);
      await destination.setValue('dest1');

      wrapper.vm.formData.conditions = []



      
      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');
      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();
      expect(alertCreateSpy).toHaveBeenCalledTimes(0);
    })
    it('should not call api when user doesnot select stream type ',async () => {
      const alertCreateSpy = vi.spyOn(alertsService, 'create');

      vi.spyOn(wrapper.vm, 'updateStreams').mockResolvedValue({ success: true });

      const mockUpdateStreamFields = vi.spyOn(wrapper.vm, 'updateStreamFields');
      const mockRemoveField = vi.fn();
      wrapper.vm.removeField = mockRemoveField;
      mockUpdateStreamFields.mockResolvedValue({ success: true })


      await flushPromises();
      const scheduledAlertComponent = await wrapper.findComponent(ScheduledAlert);

      expect(scheduledAlertComponent.exists()).toBe(true);
      flushPromises();
      const previewAlertComponent = await wrapper.findComponent(PreviewAlert);
      expect(previewAlertComponent.exists()).toBe(true);
      flushPromises();

      const fieldsInput = await wrapper.findComponent(FieldsInput);
      expect(fieldsInput.exists()).toBe(true);

      const alertNameWrapper = wrapper.find('[data-test="add-alert-name-input"]');
      await alertNameWrapper.find("input").setValue("alert123");
      

      wrapper.vm.formData.query_condition.conditions = [];



      const streamNameSelect = await wrapper.find('[data-test="add-alert-stream-select"]');
      const streamName = await streamNameSelect.findComponent({ name: 'QSelect' });
      expect(streamName.exists()).toBe(true);
      await streamName.setValue('k8s_json');
      await flushPromises();

      const destinationSelect  = await wrapper.find('[data-test="add-alert-destination-select"]');
      const destination = await destinationSelect.findComponent({ name: 'QSelect' });
      expect(destination.exists()).toBe(true);
      await destination.setValue('dest1');

      wrapper.vm.formData.conditions = []



      
      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');
      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();
      expect(alertCreateSpy).toHaveBeenCalledTimes(0);
    })
    it('should not call api when user doesnot select stream name ',async () => {
      const alertCreateSpy = vi.spyOn(alertsService, 'create');
      

      vi.spyOn(wrapper.vm, 'updateStreams').mockResolvedValue({ success: true });

      const mockUpdateStreamFields = vi.spyOn(wrapper.vm, 'updateStreamFields');
      const mockRemoveField = vi.fn();
      wrapper.vm.removeField = mockRemoveField;
      mockUpdateStreamFields.mockResolvedValue({ success: true })


      await flushPromises();
      const scheduledAlertComponent = await wrapper.findComponent(ScheduledAlert);

      expect(scheduledAlertComponent.exists()).toBe(true);
      flushPromises();

      const fieldsInput = await wrapper.findComponent(FieldsInput);
      expect(fieldsInput.exists()).toBe(true);

      const alertNameWrapper = wrapper.find('[data-test="add-alert-name-input"]');
      await alertNameWrapper.find("input").setValue("alert123");

      const streamTypeSelect = await wrapper.find('[data-test="add-alert-stream-type-select"]');
      const streamType = await streamTypeSelect.findComponent({ name: 'QSelect' });
      expect(streamType.exists()).toBe(true);
      await streamType.setValue('logs');

      await flushPromises();
      

      wrapper.vm.formData.query_condition.conditions = [];

      const destinationSelect  = await wrapper.find('[data-test="add-alert-destination-select"]');
      const destination = await destinationSelect.findComponent({ name: 'QSelect' });
      expect(destination.exists()).toBe(true);
      await destination.setValue('dest1');

      wrapper.vm.formData.conditions = []



      
      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');
      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();
      expect(alertCreateSpy).toHaveBeenCalledTimes(0);
    })
    it('should not call api when user doesnot select destination and return error with Destination Required message ',async () => {
      const alertCreateSpy = vi.spyOn(alertsService, 'create');

      vi.spyOn(wrapper.vm, 'updateStreams').mockResolvedValue({ success: true });

      const mockUpdateStreamFields = vi.spyOn(wrapper.vm, 'updateStreamFields');
      const mockRemoveField = vi.fn();
      wrapper.vm.removeField = mockRemoveField;
      mockUpdateStreamFields.mockResolvedValue({ success: true })


      await flushPromises();
      const scheduledAlertComponent = await wrapper.findComponent(ScheduledAlert);

      expect(scheduledAlertComponent.exists()).toBe(true);
      flushPromises();

      const fieldsInput = await wrapper.findComponent(FieldsInput);
      expect(fieldsInput.exists()).toBe(true);

      const alertNameWrapper = wrapper.find('[data-test="add-alert-name-input"]');
      await alertNameWrapper.find("input").setValue("alert123");

      const streamTypeSelect = await wrapper.find('[data-test="add-alert-stream-type-select"]');
      const streamType = await streamTypeSelect.findComponent({ name: 'QSelect' });
      expect(streamType.exists()).toBe(true);
      await streamType.setValue('logs');

      await flushPromises();


      const streamNameSelect = await wrapper.find('[data-test="add-alert-stream-select"]');
      const streamName = await streamNameSelect.findComponent({ name: 'QSelect' });
      expect(streamName.exists()).toBe(true);
      await streamName.setValue('k8s_json');
      await flushPromises();
      

      wrapper.vm.formData.query_condition.conditions = [];

      wrapper.vm.formData.conditions = []



      
      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');
      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();
      console.log(alertCreateSpy.mock.settledResults[0].value.response.data.data.message,'message')
      // expect(alertCreateSpy.mock.calls[0][3].destinations).toEqual([]);
      expect(alertCreateSpy.mock.settledResults[0].value.response.data.data.message).toBe("Destination Required");

    })
    it('should not call api when user doesnot give conditions value ',async () => {
      const alertCreateSpy = vi.spyOn(alertsService, 'create');

      vi.spyOn(wrapper.vm, 'updateStreams').mockResolvedValue({ success: true });

      const mockUpdateStreamFields = vi.spyOn(wrapper.vm, 'updateStreamFields');
      const mockRemoveField = vi.fn();
      wrapper.vm.removeField = mockRemoveField;
      mockUpdateStreamFields.mockResolvedValue({ success: true })


      await flushPromises();
      const scheduledAlertComponent = await wrapper.findComponent(ScheduledAlert);

      expect(scheduledAlertComponent.exists()).toBe(true);
      flushPromises();

      const fieldsInput = await wrapper.findComponent(FieldsInput);
      expect(fieldsInput.exists()).toBe(true);

      const alertNameWrapper = wrapper.find('[data-test="add-alert-name-input"]');
      await alertNameWrapper.find("input").setValue("alert123");

      const streamTypeSelect = await wrapper.find('[data-test="add-alert-stream-type-select"]');
      const streamType = await streamTypeSelect.findComponent({ name: 'QSelect' });
      expect(streamType.exists()).toBe(true);
      await streamType.setValue('logs');

      await flushPromises();
      const streamNameSelect = await wrapper.find('[data-test="add-alert-stream-select"]');
      const streamName = await streamNameSelect.findComponent({ name: 'QSelect' });
      expect(streamName.exists()).toBe(true);
      await streamName.setValue('k8s_json');
      await flushPromises();
      
      const conditionSelect = await wrapper.find('[data-test="alert-conditions-select-column"]');
      const conditionInput = await conditionSelect.findComponent({ name: 'QSelect' });
      expect(conditionInput.exists()).toBe(true);
      await conditionInput.setValue('k8s_kubernetes');
      await flushPromises();
      

      // wrapper.vm.formData.query_condition.conditions = [];

      const destinationSelect  = await wrapper.find('[data-test="add-alert-destination-select"]');
      const destination = await destinationSelect.findComponent({ name: 'QSelect' });
      expect(destination.exists()).toBe(true);
      await destination.setValue('dest1');
      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');
      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();
      expect(alertCreateSpy).toHaveBeenCalledTimes(0);
    })

    it('should validate input period before calling api and giving negative value should return false',async () => {
      const alertCreateSpy = vi.spyOn(alertsService, 'create');
      const inputvalidSpy = vi.spyOn(wrapper.vm, 'validateInputs');
      wrapper.vm.formData = {
        "name": "basic_standard_alert_10",
        "stream_type": "logs",
        "stream_name": "k8s_json",
        "is_real_time": "false",
        "query_condition": {
            "conditions": [],
            "sql": "",
            "promql": "",
            "type": "custom",
            "aggregation": {
                "group_by": [
                    ""
                ],
                "function": "avg",
                "having": {
                    "column": "",
                    "operator": ">=",
                    "value": 1
                }
            },
            "promql_condition": null,
            "vrl_function": null,
            "multi_time_range": []
        },
        "trigger_condition": {
            "period": -10,
            "operator": ">=",
            "frequency": 1,
            "cron": "",
            "threshold": 10,
            "silence": 10,
            "frequency_type": "minutes",
            "timezone": "UTC"
        },
        "destinations": [
            "ksjf"
        ],
        "context_attributes": [],
        "enabled": true,
        "description": "",
        "lastTriggeredAt": 0,
        "createdAt": "",
        "updatedAt": "2024-11-21T08:26:50.907Z",
        "owner": "",
        "lastEditedBy": ""
    }

      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');


      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();
      expect(inputvalidSpy).toHaveBeenCalledTimes(1);
      await vi.advanceTimersByTime(500);
      expect(inputvalidSpy.mock.results[0].value).toBe(false);

      expect(alertCreateSpy).toHaveBeenCalledTimes(0);
    })
    it('should validate input threshold before calling api and giving 0 as a  value should return false ',async () => {
      const alertCreateSpy = vi.spyOn(alertsService, 'create');
      const inputvalidSpy = vi.spyOn(wrapper.vm, 'validateInputs');
      wrapper.vm.formData = {
        "name": "basic_standard_alert_10",
        "stream_type": "logs",
        "stream_name": "k8s_json",
        "is_real_time": "false",
        "query_condition": {
            "conditions": [],
            "sql": "",
            "promql": "",
            "type": "custom",
            "aggregation": {
                "group_by": [
                    ""
                ],
                "function": "avg",
                "having": {
                    "column": "",
                    "operator": ">=",
                    "value": 1
                }
            },
            "promql_condition": null,
            "vrl_function": null,
            "multi_time_range": []
        },
        "trigger_condition": {
            "period": 10,
            "operator": ">=",
            "frequency": 1,
            "cron": "",
            "threshold": 0,
            "silence": 10,
            "frequency_type": "minutes",
            "timezone": "UTC"
        },
        "destinations": [
            "ksjf"
        ],
        "context_attributes": [],
        "enabled": true,
        "description": "",
        "lastTriggeredAt": 0,
        "createdAt": "",
        "updatedAt": "2024-11-21T08:26:50.907Z",
        "owner": "",
        "lastEditedBy": ""
    }

      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');


      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();
      expect(inputvalidSpy).toHaveBeenCalledTimes(1);
      await vi.advanceTimersByTime(500);
      expect(inputvalidSpy.mock.results[0].value).toBe(false);

      expect(alertCreateSpy).toHaveBeenCalledTimes(0);
    })
    it('should validate input silence notification before calling api and giving empty value should return false ',async () => {
      const alertCreateSpy = vi.spyOn(alertsService, 'create');
      const inputvalidSpy = vi.spyOn(wrapper.vm, 'validateInputs');
      wrapper.vm.formData = {
        "name": "basic_standard_alert_10",
        "stream_type": "logs",
        "stream_name": "k8s_json",
        "is_real_time": "false",
        "query_condition": {
            "conditions": [],
            "sql": "",
            "promql": "",
            "type": "custom",
            "aggregation": {
                "group_by": [
                    ""
                ],
                "function": "avg",
                "having": {
                    "column": "",
                    "operator": ">=",
                    "value": 1
                }
            },
            "promql_condition": null,
            "vrl_function": null,
            "multi_time_range": []
        },
        "trigger_condition": {
            "period": 10,
            "operator": ">=",
            "frequency": 10,
            "cron": "",
            "threshold": 10,
            "silence": "",
            "frequency_type": "minutes",
            "timezone": "UTC"
        },
        "destinations": [
            "ksjf"
        ],
        "context_attributes": [],
        "enabled": true,
        "description": "",
        "lastTriggeredAt": 0,
        "createdAt": "",
        "updatedAt": "2024-11-21T08:26:50.907Z",
        "owner": "",
        "lastEditedBy": ""
    }

      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');


      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();
      expect(inputvalidSpy).toHaveBeenCalledTimes(1);
      await vi.advanceTimersByTime(500);
      expect(inputvalidSpy.mock.results[0].value).toBe(false);

      expect(alertCreateSpy).toHaveBeenCalledTimes(0);
    })

    it('should not all selecting all columns in sql mode ', async () => {

      const onSubmitSpy = vi.spyOn(wrapper.vm, 'onSubmit');
      const getParserSpy = vi.spyOn(wrapper.vm, 'getParser');
      wrapper.vm.formData = {
        "name": "basic_standard_alert_10",
        "stream_type": "logs",
        "stream_name": "k8s_json",
        "is_real_time": "false",
        "query_condition": {
            "conditions": [],
            "sql": "select * from 'default'",
            "promql": "",
            "type": "sql",
            "aggregation": {
                "group_by": [
                    ""
                ],
                "function": "avg",
                "having": {
                    "column": "",
                    "operator": ">=",
                    "value": 1
                }
            },
            "promql_condition": null,
            "vrl_function": null,
            "multi_time_range": []
        },
        "trigger_condition": {
            "period": 10,
            "operator": ">=",
            "frequency": 10,
            "cron": "",
            "threshold": 10,
            "silence": "",
            "frequency_type": "minutes",
            "timezone": "UTC"
        },
        "destinations": [
            "ksjf"
        ],
        "context_attributes": [],
        "enabled": true,
        "description": "",
        "lastTriggeredAt": 0,
        "createdAt": "",
        "updatedAt": "2024-11-21T08:26:50.907Z",
        "owner": "",
        "lastEditedBy": ""
      }

      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');


      await flushPromises();
      await vi.advanceTimersByTime(500);
      expect(getParserSpy.mock.results[0].value).toBe(false);
      await flushPromises();
      expect(onSubmitSpy.mock.settledResults[0].value).toBe(false);
    });

    it('add aditional variables to the alert', async () => {
      const onSubmitSpy = vi.spyOn(wrapper.vm, 'onSubmit');

      wrapper.vm.formData = {
        "name": "basic_standard_alert_10",
        "stream_type": "logs",
        "stream_name": "k8s_json",
        "is_real_time": "false",
        "query_condition": {
            "conditions": [],
            "sql": "select * from 'default'",
            "promql": "",
            "type": "sql",
            "aggregation": {
                "group_by": [
                    ""
                ],
                "function": "avg",
                "having": {
                    "column": "",
                    "operator": ">=",
                    "value": 1
                }
            },
            "promql_condition": null,
            "vrl_function": null,
            "multi_time_range": []
        },
        "trigger_condition": {
            "period": 10,
            "operator": ">=",
            "frequency": 10,
            "cron": "",
            "threshold": 10,
            "silence": "",
            "frequency_type": "minutes",
            "timezone": "UTC"
        },
        "destinations": [
            "ksjf"
        ],
        "context_attributes": [],
        "enabled": true,
        "description": "",
        "lastTriggeredAt": 0,
        "createdAt": "",
        "updatedAt": "2024-11-21T08:26:50.907Z",
        "owner": "",
        "lastEditedBy": ""
      }
      const variableFieldInput = wrapper.findComponent(VariablesInput);

      expect(variableFieldInput.exists()).toBe(true);
      const btn = variableFieldInput.find('[data-test="alert-variables-add-btn"]');
      const fieldBtn = btn.find('button');
        await btn.trigger('click');
      const keyInput = await variableFieldInput.find('[data-test="alert-variables-key-input"]')
      await keyInput.setValue('key');
      const valueInput = await variableFieldInput.find('[data-test="alert-variables-value-input"]')
      await valueInput.setValue('value');

      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');


      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();
      expect(onSubmitSpy.mock.settledResults[0].value).toBe(false);


    })
    it('should convert corn expression into frequency ', async () => {
      const alertCreateSpy = vi.spyOn(alertsService, 'create');
      const onSubmitSpy = vi.spyOn(wrapper.vm, 'onSubmit');
      wrapper.vm.formData = {
        "name": "basic_standard_alert_10",
        "stream_type": "logs",
        "stream_name": "k8s_json",
        "is_real_time": "false",
        "query_condition": {
            "conditions": [],
            "sql": "",
            "promql": "",
            "type": "custom",
            "aggregation": {
                "group_by": [
                    ""
                ],
                "function": "avg",
                "having": {
                    "column": "",
                    "operator": ">=",
                    "value": 1
                }
            },
            "promql_condition": null,
            "vrl_function": null,
            "multi_time_range": []
        },
        "trigger_condition": {
            "period": -10,
            "operator": ">=",
            "frequency": 1,
            "cron": "",
            "threshold": 10,
            "silence": 10,
            "frequency_type": "cron",
            "timezone": "UTC"
        },
        "destinations": [
            "ksjf"
        ],
        "context_attributes": [],
        "enabled": true,
        "description": "",
        "lastTriggeredAt": 0,
        "createdAt": "",
        "updatedAt": "2024-11-21T08:26:50.907Z",
        "owner": "",
        "lastEditedBy": ""
    }
    const ScheduledAlertWrapper = wrapper.findComponent(ScheduledAlert);

    const cronToggleBtn = await  wrapper.find('[data-test="scheduled-alert-cron-toggle-btn"]').trigger('click');
    const cronInput = await ScheduledAlertWrapper.find('[data-test="scheduled-alert-cron-input-field"]');

    console.log(cronInput,'cronInput')
    await cronInput.setValue('1 40 * * * *')

      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');
      


      await flushPromises();  
      await vi.advanceTimersByTime(500);
      expect(wrapper.vm.formData.tz_offset).toBe(0);
      await flushPromises();
      await vi.advanceTimersByTime(500);



      expect(alertCreateSpy).toHaveBeenCalledTimes(1);
    })
    it('should throw an error when invalid sql query given', async () => {
      const onSubmitSpy = vi.spyOn(wrapper.vm, 'onSubmit');
      const addAlertFormSpy = vi.spyOn(wrapper.vm.addAlertForm, 'validate');
      const validateSqlPromise = vi.spyOn(wrapper.vm, 'validateSqlQuery');
      wrapper.vm.formData = {
        "name": "basic_standard_alert_10",
        "stream_type": "logs",
        "stream_name": "k8s_json",
        "is_real_time": "false",
        "query_condition": {
            "conditions": [],
            "sql": "select code from ''",
            "promql": "",
            "type": "sql",
            "aggregation": {
                "group_by": [
                    ""
                ],
                "function": "avg",
                "having": {
                    "column": "",
                    "operator": ">=",
                    "value": 1
                }
            },
            "promql_condition": null,
            "vrl_function": null,
            "multi_time_range": []
        },
        "trigger_condition": {
            "period": 10,
            "operator": ">=",
            "frequency": 1,
            "cron": "",
            "threshold": 10,
            "silence": 10,
            "frequency_type": "cron",
            "timezone": "UTC"
        },
        "destinations": [
            "ksjf"
        ],
        "context_attributes": [],
        "enabled": true,
        "description": "",
        "lastTriggeredAt": 0,
        "createdAt": "",
        "updatedAt": "2024-11-21T08:26:50.907Z",
        "owner": "",
        "lastEditedBy": ""
    }

            await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click');
      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');

      await flushPromises();


      await vi.advanceTimersByTime(500);

      await flushPromises();

      await vi.advanceTimersByTime(300);

      expect(onSubmitSpy.mock.settledResults[0].value).toBe(undefined);

    });

    it('should update the alert successfully', async () => {
      wrapper.vm.addAlertForm = {
        validate: vi.fn().mockResolvedValue(true), // Simulates a valid form
        resetValidation: vi.fn(), // Mock other methods if needed
      };
      const submitSpy = vi.spyOn(alertsService,'update');
      wrapper.vm.beingUpdated = true;
      wrapper.vm.formData = {
        "name": "basic_standard_alert_10",
        "stream_type": "logs",
        "stream_name": "k8s_json",
        "is_real_time": "false",
        "query_condition": {
            "conditions": [],
            "sql": "",
            "promql": "",
            "type": "custom",
            "aggregation": {
                "group_by": [
                    ""
                ],
                "function": "avg",
                "having": {
                    "column": "",
                    "operator": ">=",
                    "value": 1
                }
            },
            "promql_condition": null,
            "vrl_function": null,
            "multi_time_range": []
        },
        "trigger_condition": {
            "period": 10,
            "operator": ">=",
            "frequency": 1,
            "cron": "",
            "threshold": 3,
            "silence": 10,
            "frequency_type": "minutes",
            "timezone": "UTC"
        },
        "destinations": [
            "ksjf"
        ],
        "context_attributes": [],
        "enabled": true,
        "description": "",
        "lastTriggeredAt": 0,
        "createdAt": "",
        "updatedAt": "2024-11-21T08:26:50.907Z",
        "owner": "",
        "lastEditedBy": ""
    }


      
      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger('click')

      await wrapper.find('[data-test="add-alert-form"]').trigger('submit');
      await flushPromises();
      await vi.advanceTimersByTime(500);

      await flushPromises();

      expect(submitSpy).toHaveBeenCalledTimes(1);

    })

});




  
});
