def query(table)
  <<~SQL
    SELECT * FROM #{table}
    WHERE name = 'quoted' AND tag = '{brace}'
  SQL
end

def render(table)
  query(table)
end

puts query('users')
