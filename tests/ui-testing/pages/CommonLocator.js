// Common Locator
export var dateTimeButtonLocator='[data-test="date-time-btn"]';
export var relative30SecondsButtonLocator='[data-test="date-time-relative-30-s-btn"]';
export var absoluteTabLocator='[data-test="date-time-absolute-tab"]';
export var Past30SecondsValue='Past 30 Seconds';
export var startTimeValue='01:01:01';
export var endTimeValue='02:02:02';
export var startDateTimeValue='2024/10/01 01:01:01';
export var endDateTimeValue='2024/10/01 02:02:02';
//export var oneDateMonthLocator='//span[text() ='"1"']'; 
// export var oneDateMonthLocator='//*[@id="date-time-menu"]/div[2]/div/div/div/div[1]/div/div[2]/div/div/div[3]/div/div[3]/button/span[2]/span';
// export var oneDateMonthLocator='//span[contains(@class, "q-btn__content")]/span[text()="1"]';
export const months = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
export let month ='Oct'
export let monthSelected ='10'
export let year ='2024'
export let day = '1'
//export var oneDateMonthLocator='//span[text() ="+day+"]';
export var oneDateMonthLocator="//span[text() ='"+day+"']";
//"//span[text() ='1']"
//"//span[text() ='"+month+"']"
export let daySelected = '01'
// export let starttime = '01:01:01'
// export let endtime ='02:02:02'
export const d =new Date()
export const date = d.getDate()
export let currentYear = d.getFullYear()
export let currentMonth = months[d.getMonth()]
export let currentMon=currentMonth.substring(0, 3);
