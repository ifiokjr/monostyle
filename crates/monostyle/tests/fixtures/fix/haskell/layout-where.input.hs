module Demo where

render :: Int -> String
render n =
  let prefix = "value {braced} "
      suffix = show n
  in prefix ++ suffix

total :: Int
total = length (render 3)



main :: IO ()
main = putStrLn (render 5)
