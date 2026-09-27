#include <string>

const char* pattern = R"(\d+ {kept} "quoted")";
const std::string json = R"({
  "user": "ada",
  "roles": ["admin"]
})";

int main() {
    return json.size() > 0 ? 0 : 1;
}
