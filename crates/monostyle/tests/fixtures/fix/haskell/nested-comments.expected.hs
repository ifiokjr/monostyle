module Demo where

{- outer {braced} "quoted"
   {- nested {- deeper -} still -}
   more outer text -}
value :: String
value = "plain {braces} text"

helper :: Int -> String
helper x = "n is " ++ show x

main :: IO ()
main = putStrLn (helper 7)
