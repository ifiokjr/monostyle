namespace app::detail {

class Engine {
public:
    void start(int ticks);
    [[nodiscard]] bool running() const { return ticks_ > 0; }

private:
    int ticks_ = 0;
};

}  // namespace app::detail
