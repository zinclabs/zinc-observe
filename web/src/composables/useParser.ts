const useParser = () => {
  const sqlParser = async () => {
    const Parser: any = await import("node-sql-parser/build/mysql");

    if (Parser) {
      return new Parser.default.Parser();
    }
  };

  const sqlValidator = async () => {
  // @ts-ignore
    const Parser : any = await  import('js-sql-parser/dist/parser/sqlParser');

    console.log(Parser,"parser in validator")
    return new Parser.default.Parser();
  }
    
    return {
    sqlParser,
    sqlValidator
  };
};

export default useParser;
