import DatafusionSQL from "@openobserve/node-sql-parser/build/DatafusionSQL";
const useParser = () => {
  const sqlParser = async () => {
    console.log(DatafusionSQL, "DatafusionSQL");
    const Parser: any = DatafusionSQL.Parser;

    if (Parser) {
      return new Parser.default.Parser();
    }
  };

  return {
    sqlParser,
  };
};

export default useParser;
