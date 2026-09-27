#include <map>
#include <string>
#include <vector>

template <typename T, typename U>
using Table = std::map<T, std::vector<U>>;

int main() {
    Table<std::string, int> table;

    table["alpha {kept}"] = {1, 2, 3};

    return 0;
}
