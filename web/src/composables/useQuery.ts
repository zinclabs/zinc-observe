import { useStore } from "vuex";
import useNotifications from "@/composables/useNotifications";
import { b64EncodeUnicode } from "@/utils/zincutils";
import { onBeforeMount } from "vue";

interface BuildQueryPayload {
  from?: number;
  size?: number;
  timestamp_column?: string;
  timestamps?: {
    startTime: number | "Invalid Date";
    endTime: number | "Invalid Date";
  };
  timeInterval?: string;
  sqlMode: boolean;
  currentPage?: number;
  selectedStream?: string;
  parsedQuery?: {
    queryFunctions: string;
    whereClause: string;
    limit: number;
    query: string;
    offset: number;
  };
  streamName: "";
}

const getTimeInterval = (start_time: number, end_time: number) => {
  const resPayload = {
    interval: "10 second",
    keyFormat: "HH:mm:ss",
  };

  if (end_time - start_time >= 1000000 * 60 * 30) {
    resPayload.interval = "15 second";
    resPayload.keyFormat = "HH:mm:ss";
  }
  if (end_time - start_time >= 1000000 * 60 * 60) {
    resPayload.interval = "30 second";
    resPayload.keyFormat = "HH:mm:ss";
  }
  if (end_time - start_time >= 1000000 * 3600 * 2) {
    resPayload.interval = "1 minute";
    resPayload.keyFormat = "MM-DD HH:mm";
  }
  if (end_time - start_time >= 1000000 * 3600 * 6) {
    resPayload.interval = "5 minute";
    resPayload.keyFormat = "MM-DD HH:mm";
  }
  if (end_time - start_time >= 1000000 * 3600 * 24) {
    resPayload.interval = "30 minute";
    resPayload.keyFormat = "MM-DD HH:mm";
  }
  if (end_time - start_time >= 1000000 * 86400 * 7) {
    resPayload.interval = "1 hour";
    resPayload.keyFormat = "MM-DD HH:mm";
  }
  if (end_time - start_time >= 1000000 * 86400 * 30) {
    resPayload.interval = "1 day";
    resPayload.keyFormat = "YYYY-MM-DD";
  }
  return resPayload;
};

const useQuery = () => {
  let parser: any;
  const store = useStore();
  const { showErrorNotification } = useNotifications();

  onBeforeMount(async () => {
    await importSqlParser();
  });

  const importSqlParser = async () => {
    const useSqlParser: any = await import("@/composables/useParser");
    const { sqlParser }: any = useSqlParser.default();
    parser = await sqlParser();
  };

  const parseQuery = (query: string, sqlMode = false) => {
    const parsedParams = {
      queryFunctions: "",
      whereClause: "",
      limit: null,
      query: query,
      offset: null,
    };
    if (sqlMode == true) {
      const parsedSQL: any = parser.astify(query);
      if (parsedSQL.limit != null) {
        parsedParams.limit = parsedSQL.limit.value[0].value;

        if (parsedSQL.limit.seperator == "offset") {
          parsedParams.offset = parsedSQL.limit.value[1].value || 0;
        }

        parsedSQL.limit = null;

        parsedParams.query = parser.sqlify(parsedSQL);

        //replace backticks with \" for sql_mode
        parsedParams.query = parsedParams.query.replace(/`/g, '"');
      }
    } else {
      const parseQuery = parsedParams.query.split("|");
      let queryFunctions = "";
      let whereClause = "";
      if (parseQuery.length > 1) {
        queryFunctions = "," + parseQuery[0].trim();
        whereClause = parseQuery[1].trim();
      } else {
        whereClause = parseQuery[0].trim();
      }

      if (whereClause.trim() != "") {
        whereClause = whereClause
          .replace(/=(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " =")
          .replace(/>(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " >")
          .replace(/<(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " <");

        whereClause = whereClause
          .replace(/!=(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " !=")
          .replace(/! =(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " !=")
          .replace(/< =(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " <=")
          .replace(/> =(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " >=");

        // const parsedSQL = whereClause.split(" ");
        // searchObj.data.stream.selectedStreamFields.forEach((field: any) => {
        //   parsedSQL.forEach((node: any, index: any) => {
        //     if (node == field.name) {
        //       node = node.replaceAll('"', "");
        //       parsedSQL[index] = '"' + node + '"';
        //     }
        //   });
        // });

        // whereClause = parsedSQL.join(" ");

        parsedParams.whereClause = whereClause;
      }

      parsedParams.queryFunctions = queryFunctions;
    }
  };

  const buildQueryPayload = (data: BuildQueryPayload) => {
    try {
      const req: any = {
        query: {
          sql: 'select *[QUERY_FUNCTIONS] from "[INDEX_NAME]" [WHERE_CLAUSE]',
          start_time: (new Date().getTime() - 900000) * 1000,
          end_time: new Date().getTime() * 1000,
          from: data.from || 0,
          size: data.size || 100,
        },
        aggs: {
          histogram:
            "select histogram(" +
            (data?.timestamp_column ||
              store.state.zoConfig.timestamp_column ||
              "_timestamp") +
            ", '[INTERVAL]') AS zo_sql_key, count(*) AS zo_sql_num from query GROUP BY zo_sql_key ORDER BY zo_sql_key",
        },
      };

      req.aggs.histogram = req.aggs.histogram.replaceAll(
        "[INTERVAL]",
        data.timeInterval,
      );

      req.query.sql = req.query.sql.replaceAll(
        "[QUERY_FUNCTIONS]",
        data.parsedQuery?.queryFunctions || "",
      );

      req.query.sql = req.query.sql.replaceAll("[INDEX_NAME]", data.streamName);

      req.query.sql = req.query.sql.replaceAll(
        "[WHERE_CLAUSE]",
        data.parsedQuery?.whereClause || "",
      );

      if (data?.timestamps?.startTime)
        req.query.start_time = data.timestamps.startTime;

      if (data?.timestamps?.endTime)
        req.query.end_time = data.timestamps.endTime;

      if (store.state.zoConfig.sql_base64_enabled) {
        req["encoding"] = "base64";
        req.query.sql = b64EncodeUnicode(req.query.sql);
        if (!data.sqlMode && data.currentPage == 0) {
          req.aggs.histogram = b64EncodeUnicode(req.aggs.histogram);
        }
      }

      return req;
    } catch (e: any) {
      showErrorNotification("Invalid SQL Syntax");
    }
  };

  return { getTimeInterval, parseQuery, buildQueryPayload };
};

export default useQuery;
