-- Registry of scanned targets.
local function scan(target)
    -- Skip when the target is missing.
    if not target then
        return nil
    end


    return target:read()
end

return scan
