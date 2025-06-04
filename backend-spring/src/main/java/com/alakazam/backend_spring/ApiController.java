package com.alakazam.backend_spring;

import com.alakazam.backend_spring.data.Search;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;

import java.util.List;
import java.util.Map;

@RestController
public class ApiController {
    @Autowired
    private Search search;

    @PostMapping("/search")
    public List<Search.MatchResultDetailed> search(@RequestBody Map<String, Object> body) {
        List<String> hashes = (List<String>) body.get("hashes");
        long[] hashArray = hashes.stream().mapToLong(Long::parseLong).toArray();
        return search.searchRedis(hashArray);
    }
}
