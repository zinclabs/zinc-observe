export const convertBillingData = (params:any) => {

          let series: any = [];
          let eventIndexMap: any = [];
          if (params.data?.length > 0) {
            params.data.forEach(
              (data: any) => {
                let eventIndex = eventIndexMap.indexOf(data.event);
                if (eventIndex > -1) {
                  if (series[eventIndex] === undefined) {
                    series[eventIndex] = {
                      data:[],
                      name: data.event,
                      type: "bar",
                      emphasis: { focus: "series" }
                    };
                  }
                  series[eventIndex].data.push([data.usage_timestamp,Math.round(parseInt(data.size) / 1024 / 1024)]);
                } else {
                  // If the event value is not found, add it to eventIndexMap and chartObj
                  // let newIndex = eventIndexMap.length;
                  eventIndexMap.push(data.event);
                  eventIndex = eventIndexMap.indexOf(data.event);
                  series[eventIndex] = {
                    data:[],
                    name: data.event,
                    type: "bar",
                    emphasis: { focus: "series" }
                  };
                  
                  // Update the newly added index with the data values
                  series[eventIndex].data.push([data.usage_timestamp,Math.round(parseInt(data.size) / 1024 / 1024)]);
                }
              }
            );
          }
  
  const options: any = {
    backgroundColor: "transparent",
    grid: {
      containLabel: true,
      left: "20",
      right:"200",
      top: "30",
      bottom: "0",
    },
    legend:{
      show: true,
      type: "scroll",
      orient: "vertical",
      left:null,
      right:0,
      bottom:"ceenter",
      padding: [10, 20, 10, 10],
      tooltip: {
        show: true,
        padding: 10,
        textStyle: {
          fontSize: 12,
        },
        backgroundColor: "rgba(255,255,255,0.8)",
      },
      textStyle: {
        width: 100,
        overflow: "truncate",
      },
    },
    tooltip: {
      show:true,
      trigger: "axis",
      textStyle: {
        fontSize: 12,
      },
      axisPointer: {
        type: "cross",
        label: {
          show: true,
          fontsize: 12,
          formatter: function (params: any) {
            if (params.axisDimension == "y") {
              return params?.value?.toFixed(2) + "MB";
            } else {
              return `${formatDate(new Date(params.value))}`;
            }
          },
        },
      },
      formatter: function (name: any) {
        if (name?.length == 0) return "";
        const date = new Date(name?.[0]?.data?.[0]);
        return `${formatDate(date)} <br/> ${name?.map((item: any) => `${item?.marker} ${item?.seriesName} : <b>${item?.value?.[1]}MB</b>`).join("<br/>")}`;
      },
    },
    xAxis: {
      type: "time",
    },
    yAxis: {
      type: "value",
      axisLine: {
        show: true,
      },
      axisLabel:{
        formatter: function (value: any) {
          return `${value}MB`
        }
      }
    },
    series: series,
   }   
    return {options};
}

const formatDate = (date: any) => {
  const year = String(date.getFullYear());
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  const seconds = String(date.getSeconds()).padStart(2, "0");

  return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`;
}