def routes
  api = %r{^/api/v\d+$}
  nested = %r{/user/{\d+}}


  [api, nested]
end
