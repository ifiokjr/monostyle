public class Query {
    private static final String SQL = """
        {
          "table": "users",
          "limit": %d
        }
        """;



    public String build(int limit) {
        return SQL.formatted(limit);
    }
}
