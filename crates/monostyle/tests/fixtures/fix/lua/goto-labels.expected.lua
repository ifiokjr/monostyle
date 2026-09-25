local function scan(needle, values)

    for _, value in ipairs(values) do
        if value == needle then
            goto found
        end
    end

    do return "missing" end

    ::found::
    do return "found" end
end

return scan
