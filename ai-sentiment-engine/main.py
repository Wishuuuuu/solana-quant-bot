import asyncio
import json
import os
from datetime import datetime, timedelta
from typing import Dict, List, Optional
import aiohttp
import redis.asyncio as redis
from vaderSentiment.vaderSentiment import SentimentIntensityAnalyzer
from dataclasses import dataclass
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

@dataclass
class SentimentResult:
    token_mint: str
    hype_score: float
    sentiment: str
    volume_velocity: float
    spam_score: float
    timestamp: datetime

class SentimentAnalyzer:
    def __init__(self):
        self.vader = SentimentIntensityAnalyzer()
        self.hype_keywords = [
            'moon', 'gem', '100x', 'pump', 'rocket', 'to the moon',
            'explosive', 'early', 'presale', 'launch', 'gem', 'alpha',
            'massive', 'huge', 'insane', 'crazy', 'bullish', 'diamond hands'
        ]
        self.spam_patterns = [
            'buy now', 'limited time', 'guaranteed', '100% sure',
            'dont miss', 'last chance', 'click here', 'link in bio'
        ]

    def analyze_text(self, text: str) -> Dict[str, float]:
        vader_scores = self.vader.polarity_scores(text)
        
        hype_count = sum(1 for keyword in self.hype_keywords if keyword.lower() in text.lower())
        spam_count = sum(1 for pattern in self.spam_patterns if pattern.lower() in text.lower())
        
        return {
            'sentiment': vader_scores['compound'],
            'hype_intensity': min(hype_count / 5.0, 1.0),
            'spam_likelihood': min(spam_count / 3.0, 1.0)
        }

class TwitterScraper:
    def __init__(self, bearer_token: Optional[str] = None):
        self.bearer_token = bearer_token or os.getenv('TWITTER_BEARER_TOKEN')
        self.base_url = "https://api.twitter.com/2"

    async def search_tweets(self, query: str, limit: int = 100) -> List[Dict]:
        if not self.bearer_token:
            logger.warning("No Twitter bearer token, using mock data")
            return self._generate_mock_tweets(query, limit)

        headers = {"Authorization": f"Bearer {self.bearer_token}"}
        params = {
            "query": query,
            "max_results": min(limit, 100),
            "tweet.fields": "created_at,public_metrics,author_id"
        }

        async with aiohttp.ClientSession() as session:
            async with session.get(
                f"{self.base_url}/tweets/search/recent",
                headers=headers,
                params=params
            ) as response:
                if response.status == 200:
                    data = await response.json()
                    return data.get('data', [])
                else:
                    logger.error(f"Twitter API error: {response.status}")
                    return self._generate_mock_tweets(query, limit)

    def _generate_mock_tweets(self, query: str, limit: int) -> List[Dict]:
        mock_tweets = []
        for i in range(limit):
            mock_tweets.append({
                'id': f'mock_{i}',
                'text': f'{query} is going to the moon! 🚀 This gem is about to explode 100x #Solana',
                'created_at': datetime.now().isoformat(),
                'public_metrics': {
                    'like_count': 100 + i * 10,
                    'retweet_count': 50 + i * 5,
                    'reply_count': 20 + i * 2
                }
            })
        return mock_tweets

class PumpFunScraper:
    def __init__(self):
        self.base_url = "https://api.pump.fun"

    async def get_token_info(self, token_mint: str) -> Dict:
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(f"{self.base_url}/tokens/{token_mint}") as response:
                    if response.status == 200:
                        return await response.json()
            except Exception as e:
                logger.error(f"Pump.fun API error: {e}")
        
        return self._generate_mock_token_info(token_mint)

    def _generate_mock_token_info(self, token_mint: str) -> Dict:
        return {
            'mint': token_mint,
            'name': 'Mock Token',
            'symbol': 'MOCK',
            'liquidity': 50000,
            'market_cap': 100000,
            'holders': 500,
            'volume_24h': 25000
        }

class AISentimentEngine:
    def __init__(self):
        self.analyzer = SentimentAnalyzer()
        self.twitter_scraper = TwitterScraper()
        self.pumpfun_scraper = PumpFunScraper()
        self.redis: Optional[redis.Redis] = None
        self.sentiment_cache: Dict[str, List[Dict]] = {}

    async def connect_redis(self):
        self.redis = await redis.from_url("redis://127.0.0.1/")
        logger.info("Connected to Redis")

    async def analyze_token(self, token_mint: str) -> SentimentResult:
        logger.info(f"Analyzing sentiment for token: {token_mint}")
        
        tweets = await self.twitter_scraper.search_tweets(token_mint, limit=50)
        token_info = await self.pumpfun_scraper.get_token_info(token_mint)
        
        sentiment_scores = []
        for tweet in tweets:
            text = tweet.get('text', '')
            scores = self.analyzer.analyze_text(text)
            sentiment_scores.append(scores)
        
        if sentiment_scores:
            avg_sentiment = sum(s['sentiment'] for s in sentiment_scores) / len(sentiment_scores)
            avg_hype = sum(s['hype_intensity'] for s in sentiment_scores) / len(sentiment_scores)
            avg_spam = sum(s['spam_likelihood'] for s in sentiment_scores) / len(sentiment_scores)
        else:
            avg_sentiment = 0.0
            avg_hype = 0.0
            avg_spam = 0.0

        volume_velocity = self._calculate_volume_velocity(tweets)
        hype_score = self._calculate_hype_score(avg_sentiment, avg_hype, volume_velocity, avg_spam)
        
        sentiment_label = self._classify_sentiment(avg_sentiment)
        
        result = SentimentResult(
            token_mint=token_mint,
            hype_score=hype_score,
            sentiment=sentiment_label,
            volume_velocity=volume_velocity,
            spam_score=avg_spam,
            timestamp=datetime.now()
        )
        
        if self.redis:
            await self.redis.publish(
                "sentiment_results",
                json.dumps({
                    'token_mint': token_mint,
                    'hype_score': hype_score,
                    'sentiment': sentiment_label,
                    'volume_velocity': volume_velocity,
                    'spam_score': avg_spam
                })
            )
        
        return result

    def _calculate_volume_velocity(self, tweets: List[Dict]) -> float:
        if not tweets:
            return 0.0
        
        now = datetime.now()
        recent_tweets = [t for t in tweets if self._is_recent(t.get('created_at', ''), now, minutes=5)]
        
        return min(len(recent_tweets) / 10.0, 1.0)

    def _is_recent(self, created_at: str, now: datetime, minutes: int) -> bool:
        try:
            tweet_time = datetime.fromisoformat(created_at.replace('Z', '+00:00'))
            return (now - tweet_time).total_seconds() <= minutes * 60
        except:
            return False

    def _calculate_hype_score(self, sentiment: float, hype: float, velocity: float, spam: float) -> float:
        base_score = (sentiment + 1) / 2 * 0.3
        hype_contribution = hype * 0.4
        velocity_contribution = velocity * 0.3
        spam_penalty = spam * 0.5
        
        final_score = base_score + hype_contribution + velocity_contribution - spam_penalty
        return max(0.0, min(1.0, final_score))

    def _classify_sentiment(self, score: float) -> str:
        if score > 0.3:
            return "Bullish"
        elif score < -0.3:
            return "Bearish"
        else:
            return "Neutral"

    async def listen_smart_money_signals(self):
        if not self.redis:
            await self.connect_redis()
        
        pubsub = self.redis.pubsub()
        await pubsub.subscribe("smart_money_signals")
        
        logger.info("Listening for smart money signals...")
        
        async for message in pubsub.listen():
            if message['type'] == 'message':
                try:
                    signal = json.loads(message['data'])
                    token_mint = signal.get('token_mint')
                    
                    logger.info(f"Received signal for token: {token_mint}")
                    
                    sentiment = await self.analyze_token(token_mint)
                    logger.info(f"Sentiment analysis complete: {sentiment}")
                    
                except Exception as e:
                    logger.error(f"Error processing signal: {e}")

async def main():
    engine = AISentimentEngine()
    await engine.connect_redis()
    
    await engine.listen_smart_money_signals()

if __name__ == "__main__":
    asyncio.run(main())
