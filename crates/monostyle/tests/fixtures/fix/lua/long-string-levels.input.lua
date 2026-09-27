local function docs()
    local nested = [=[ contains [[ and ]] inside ]=]
    local simple = [[ {kept} and "quotes" ]]


    return nested .. simple
end

return docs
