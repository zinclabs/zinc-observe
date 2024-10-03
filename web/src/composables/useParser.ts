const useParser = () => {
  const sqlParser = async () => {
    const Parser: any = await import(
      "@openobserve/node-sql-parser/build/DatafusionSQL"
    );

    if (Parser) {
      return new Parser.default.Parser();
    }
  };

  return {
    sqlParser,
  };
};

export default useParser;
